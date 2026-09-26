//! `WritingReviewer` for any OpenAI-compatible `/chat/completions` endpoint,
//! such as the university gateway (gen.ai.kku.ac.th). The model is chosen per
//! call, so every model the gateway offers works with this one adapter.

use std::time::{Duration, Instant};

use async_trait::async_trait;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::application::ports::WritingReviewer;
use crate::domain::error::{AppError, AppResult};
use crate::domain::feedback::{AiFeedback, CallTrace, ReviewOutcome, ReviewRequest};
use crate::domain::image::StoredImage;
use crate::domain::writing::WritingKind;

const SYSTEM_PROMPT: &str = r#"You are an experienced IELTS Academic Writing examiner helping a learner improve their own writing.

Assess the essay with the four official criteria:
- task: Task Achievement (Task 1) or Task Response (Task 2)
- coherence: Coherence and Cohesion
- lexical: Lexical Resource
- grammar: Grammatical Range and Accuracy
Give each an estimated band from 0 to 9 in steps of 0.5, and an overall band. Take the question and the word count into account.

Then list up to 12 concrete issues, most important first. For each issue:
- "quote": copy the exact words from the essay (at most 12 words) so they can be found in the text
- "tag": a short lower-case category. Reuse one of the learner's existing mistake tags when it fits; otherwise make a short new one such as "missing plural -s" or "article (a / the)"
- "hint": one question that helps the learner find the fix themselves. Never give the corrected wording.

Finally, add up to 3 open questions about the ideas, examples or structure.

Reply with only this JSON object and nothing else:
{"bands":{"task":0,"coherence":0,"lexical":0,"grammar":0,"overall":0},"summary":"two or three sentences","issues":[{"quote":"","tag":"","hint":""}],"questions":[""]}"#;

/// Upper limit for one reply. The answer itself is a few hundred tokens, but
/// "thinking" models (Gemini) spend part of this budget on hidden reasoning
/// first; with 1500 their JSON was cut off. A limit only caps usage, it
/// doesn't reserve quota.
const MAX_TOKENS: u32 = 4000;

/// Largest gateway response kept in the AI log.
const MAX_LOGGED_REPLY: usize = 100_000;

pub struct ChatCompletionsReviewer {
    client: reqwest::Client,
    url: String,
    api_key: String,
}

impl ChatCompletionsReviewer {
    pub fn new(base_url: &str, api_key: String) -> Result<Self, reqwest::Error> {
        let client = reqwest::Client::builder()
            // Reviewing an essay can take 10-30 s.
            .timeout(Duration::from_secs(90))
            // Keep no idle connections: calls are rare, and ntex runs a
            // separate async runtime per worker thread, so a pooled
            // connection could belong to another worker's runtime.
            .pool_max_idle_per_host(0)
            .build()?;
        Ok(Self {
            client,
            url: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            api_key,
        })
    }

    /// One request/response round trip. Returns the model's reply text and
    /// adds the tokens to `trace`. Whatever happens, `trace` ends up holding
    /// what was sent and what came back, so failed calls can be looked at too.
    async fn complete(
        &self,
        model: &str,
        messages: &[Message],
        trace: &mut CallTrace,
    ) -> AppResult<String> {
        let body = ChatRequest {
            model,
            messages,
            max_tokens: MAX_TOKENS,
            temperature: 0.2,
        };
        trace.attempts += 1;
        trace.request_json = loggable_request(&body);
        trace.raw_reply = None;
        trace.http_status = None;

        let response = self
            .client
            .post(&self.url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| {
                log::warn!("AI gateway request failed: {err}");
                if err.is_timeout() {
                    AppError::Unavailable("the AI gateway did not answer within 90 seconds".into())
                } else {
                    AppError::Unavailable("could not reach the AI gateway".into())
                }
            })?;

        let status = response.status();
        trace.http_status = Some(status.as_u16());
        // Read the body as text first, so the log gets it exactly as it came.
        let body_text = response.text().await.map_err(|err| {
            log::warn!("AI gateway response could not be read: {err}");
            AppError::Unavailable("the AI gateway's answer broke off".into())
        })?;
        trace.raw_reply = Some(capped(&body_text, MAX_LOGGED_REPLY));

        if !status.is_success() {
            let snippet: String = body_text.chars().take(200).collect();
            log::warn!("AI gateway answered {status}: {snippet}");
            return Err(match status.as_u16() {
                429 => AppError::RateLimited(format!(
                    "the gateway's limit for {model} is reached ({snippet}); quotas reset at midnight"
                )),
                401 | 403 => AppError::Unavailable("the AI gateway rejected the API key".into()),
                _ => AppError::Unavailable(format!("the AI gateway answered {status}")),
            });
        }

        let parsed: ChatResponse = serde_json::from_str(&body_text).map_err(|err| {
            log::warn!("unexpected AI gateway response: {err}");
            AppError::Unavailable("the AI gateway sent a response in an unexpected format".into())
        })?;

        // Some gateway errors arrive as HTTP 200 with an "error" field and
        // no choices (e.g. a model that can't take images).
        if parsed.choices.is_empty() {
            let message = parsed
                .error
                .as_ref()
                .map(gateway_error_message)
                .unwrap_or_else(|| "empty reply".into());
            log::warn!("AI gateway returned no choices for {model}: {message}");
            return Err(AppError::Unavailable(format!(
                "the gateway could not run {model}: {message}"
            )));
        }

        let reply = parsed
            .choices
            .into_iter()
            .next()
            .and_then(|choice| choice.message.content)
            .unwrap_or_default();

        // Most gateways report usage; if one doesn't, estimate ~4 characters
        // per token so the quota counter still moves.
        let (input_tokens, output_tokens) = match parsed.usage {
            Some(usage) => (usage.prompt_tokens, usage.completion_tokens),
            None => {
                let sent: usize = messages.iter().map(|m| m.content.text_len()).sum();
                ((sent / 4) as i64, (reply.len() / 4) as i64)
            }
        };
        trace.input_tokens += input_tokens;
        trace.output_tokens += output_tokens;
        Ok(reply)
    }

    /// Asks once. If the reply isn't usable, shows the model its reply and
    /// the problem, and asks once more; a second failure is reported.
    async fn review_with_retry(
        &self,
        model: &str,
        request: &ReviewRequest,
        trace: &mut CallTrace,
    ) -> AppResult<AiFeedback> {
        let mut messages = vec![
            Message::new("system", SYSTEM_PROMPT),
            Message::with_image(&user_message(request), request.image.as_ref()),
        ];

        let first = self.complete(model, &messages, trace).await?;
        let problem = match AiFeedback::from_model_reply(&first) {
            Ok(feedback) => return Ok(feedback),
            Err(problem) => problem,
        };

        log::warn!("AI reply unusable ({problem}); retrying once");
        messages.push(Message::new("assistant", &first));
        messages.push(Message::new(
            "user",
            &format!(
                "That reply could not be used: {problem}. Reply again with only the JSON object in the required shape."
            ),
        ));
        let second = self.complete(model, &messages, trace).await?;
        AiFeedback::from_model_reply(&second).map_err(|reason| {
            AppError::Unavailable(format!(
                "the model's reply could not be used ({reason}); try again or pick another model"
            ))
        })
    }
}

#[async_trait]
impl WritingReviewer for ChatCompletionsReviewer {
    async fn review(&self, model: &str, request: &ReviewRequest) -> ReviewOutcome {
        let started = Instant::now();
        let mut trace = CallTrace::default();
        let feedback = self.review_with_retry(model, request, &mut trace).await;
        trace.duration_ms = started.elapsed().as_millis() as i64;
        ReviewOutcome { feedback, trace }
    }
}

/// The request body for the AI log: the same JSON, except that each image's
/// base64 data becomes its type, size and SHA-256. A row stays a few KB,
/// and the hash still tells which image was sent.
fn loggable_request(body: &ChatRequest) -> String {
    let Ok(mut value) = serde_json::to_value(body) else {
        return String::new();
    };
    if let Some(messages) = value["messages"].as_array_mut() {
        let parts = messages
            .iter_mut()
            .filter_map(|message| message["content"].as_array_mut())
            .flatten();
        for part in parts {
            if let Some(url) = part.pointer_mut("/image_url/url")
                && let Some(described) = url.as_str().and_then(describe_data_url)
            {
                *url = described.into();
            }
        }
    }
    value.to_string()
}

/// "data:image/png;base64,iVBOR..." -> "[image/png, 49 KB, sha256 1a2b...]"
fn describe_data_url(url: &str) -> Option<String> {
    let (mime, data) = url.strip_prefix("data:")?.split_once(";base64,")?;
    let bytes = BASE64.decode(data).ok()?;
    let hash = Sha256::digest(&bytes);
    Some(format!(
        "[{mime}, {} KB, sha256 {hash:x}]",
        bytes.len().div_ceil(1024)
    ))
}

/// Keeps at most `max` bytes, cut at a character boundary. A gateway error
/// page could in theory be huge; a real reply is a few KB.
fn capped(text: &str, max: usize) -> String {
    if text.len() <= max {
        return text.to_string();
    }
    let mut end = max;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}… [cut, {} bytes in total]", &text[..end], text.len())
}

/// The part of the prompt that changes per essay.
fn user_message(request: &ReviewRequest) -> String {
    let task = match request.kind {
        WritingKind::Task1 => "IELTS Academic Writing Task 1 (at least 150 words, 20 minutes)",
        WritingKind::Task2 => "IELTS Academic Writing Task 2 (at least 250 words, 40 minutes)",
        WritingKind::Paragraph => {
            "Free paragraph practice for IELTS Academic Writing (no word minimum)"
        }
    };
    let question = if request.prompt.trim().is_empty() {
        "(no question given)"
    } else {
        request.prompt.trim()
    };
    let tags = if request.known_tags.is_empty() {
        "none yet".to_string()
    } else {
        request.known_tags.join(", ")
    };
    // Task 1 describes a chart. Without the image the model must not guess
    // whether the learner's numbers are right.
    let chart = match (request.kind, request.image.is_some()) {
        (_, true) => {
            "\n\nThe chart for this task is attached as an image. Check the learner's figures, comparisons and overview against it."
        }
        (WritingKind::Task1, false) => {
            "\n\nYou cannot see the chart for this task. Do not judge whether the figures are correct; judge the language, the structure and whether there is a clear overview."
        }
        _ => "",
    };
    format!(
        "{task}\n\nQuestion:\n{question}{chart}\n\nEssay (version {}, {} words):\n{}\n\nLearner's existing mistake tags: {tags}",
        request.version_no, request.word_count, request.essay
    )
}

// ---- JSON shapes of the chat/completions API ----------------------------

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [Message],
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: Content,
}

/// A message is either plain text or, when an image goes along, a list of
/// parts. `untagged` writes each variant as its bare value: a string, or
/// an array of parts.
#[derive(Serialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Parts(Vec<Part>),
}

impl Content {
    /// Characters of text, for estimating tokens when the gateway reports none.
    fn text_len(&self) -> usize {
        match self {
            Content::Text(text) => text.len(),
            Content::Parts(parts) => parts
                .iter()
                .map(|part| match part {
                    Part::Text { text } => text.len(),
                    Part::ImageUrl { .. } => 0,
                })
                .sum(),
        }
    }
}

/// `tag = "type"` adds `"type": "text"` or `"type": "image_url"` to each
/// part, which is the shape the chat/completions API expects:
/// `{"type":"image_url","image_url":{"url":"data:image/png;base64,..."}}`.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Part {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
}

impl Message {
    fn new(role: &str, content: &str) -> Self {
        Self {
            role: role.into(),
            content: Content::Text(content.into()),
        }
    }

    /// A user message, with the image (as a data URL) after the text if there is one.
    fn with_image(text: &str, image: Option<&StoredImage>) -> Self {
        let Some(image) = image else {
            return Self::new("user", text);
        };
        let url = format!("data:{};base64,{}", image.mime, BASE64.encode(&image.data));
        Self {
            role: "user".into(),
            content: Content::Parts(vec![
                Part::Text { text: text.into() },
                Part::ImageUrl {
                    image_url: ImageUrl { url },
                },
            ]),
        }
    }
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<Choice>,
    usage: Option<Usage>,
    /// Either `"text"` or `{"message": "text", ...}` depending on the gateway.
    error: Option<serde_json::Value>,
}

/// The readable part of a gateway error, whichever shape it came in.
fn gateway_error_message(error: &serde_json::Value) -> String {
    let text = match error {
        serde_json::Value::String(text) => text.clone(),
        other => other
            .get("message")
            .and_then(|m| m.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| other.to_string()),
    };
    text.chars().take(200).collect()
}

#[derive(Deserialize)]
struct Choice {
    message: ReplyMessage,
}

/// `Option`: some gateways send `null` content.
#[derive(Deserialize)]
struct ReplyMessage {
    content: Option<String>,
}

/// OpenAI names these prompt/completion tokens; Anthropic-style gateways
/// say input/output. `alias` accepts either.
#[derive(Deserialize)]
struct Usage {
    #[serde(alias = "input_tokens")]
    prompt_tokens: i64,
    #[serde(alias = "output_tokens")]
    completion_tokens: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_includes_question_essay_and_tags() {
        let request = ReviewRequest {
            kind: WritingKind::Task2,
            prompt: "Do universities focus too much on jobs?".into(),
            essay: "Universities play a key role.".into(),
            version_no: 2,
            word_count: 5,
            known_tags: vec!["missing plural -s".into(), "article (a / the)".into()],
            image: None,
        };
        let text = user_message(&request);
        assert!(text.contains("Task 2 (at least 250 words"));
        assert!(text.contains("Do universities focus too much on jobs?"));
        assert!(text.contains("Essay (version 2, 5 words):\nUniversities play a key role."));
        assert!(text.contains("missing plural -s, article (a / the)"));
        assert!(!text.contains("chart"));
    }

    #[test]
    fn task1_says_whether_the_chart_is_attached() {
        let mut request = ReviewRequest {
            kind: WritingKind::Task1,
            prompt: "The chart shows rainfall.".into(),
            essay: "Rainfall rose.".into(),
            version_no: 1,
            word_count: 2,
            known_tags: vec![],
            image: None,
        };
        assert!(user_message(&request).contains("You cannot see the chart"));
        request.image = Some(StoredImage {
            mime: "image/png".into(),
            data: vec![1, 2, 3],
        });
        assert!(user_message(&request).contains("attached as an image"));
    }

    #[test]
    fn image_is_sent_as_a_data_url_part() {
        let image = StoredImage {
            mime: "image/png".into(),
            data: b"png".to_vec(),
        };
        let json = serde_json::to_value(Message::with_image("Essay", Some(&image))).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "role": "user",
                "content": [
                    {"type": "text", "text": "Essay"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,cG5n"}}
                ]
            })
        );
        // Without an image the content stays a plain string.
        let json = serde_json::to_value(Message::with_image("Essay", None)).unwrap();
        assert_eq!(json["content"], "Essay");
    }

    #[test]
    fn error_in_a_200_response_is_readable() {
        let body = r#"{"status":404,"error":{"message":"No endpoints found that support image input","code":404}}"#;
        let parsed: ChatResponse = serde_json::from_str(body).unwrap();
        assert!(parsed.choices.is_empty());
        assert_eq!(
            gateway_error_message(parsed.error.as_ref().unwrap()),
            "No endpoints found that support image input"
        );
        let plain = serde_json::json!("Invalid API key");
        assert_eq!(gateway_error_message(&plain), "Invalid API key");
    }

    #[test]
    fn logged_request_describes_the_image_instead_of_its_data() {
        let image = StoredImage {
            mime: "image/png".into(),
            data: b"png".to_vec(),
        };
        let messages = [
            Message::new("system", "Be kind."),
            Message::with_image("Essay", Some(&image)),
        ];
        let body = ChatRequest {
            model: "claude-sonnet-5",
            messages: &messages,
            max_tokens: 10,
            temperature: 0.2,
        };
        let logged: serde_json::Value = serde_json::from_str(&loggable_request(&body)).unwrap();
        // sha256 of "png", as `printf png | sha256sum` prints it.
        assert_eq!(
            logged["messages"][1]["content"][1]["image_url"]["url"],
            "[image/png, 1 KB, sha256 8f8cbb7dcf46e0bc7d53265749a6c17d116093a6ba95e442764060c76fd4a86c]"
        );
        assert_eq!(logged["messages"][1]["content"][0]["text"], "Essay");
        assert_eq!(logged["model"], "claude-sonnet-5");
    }

    #[test]
    fn capped_cuts_at_a_character_boundary() {
        assert_eq!(capped("short", 10), "short");
        let cut = capped("ก่อน", 4); // Thai letters are 3 bytes each
        assert!(cut.starts_with("ก…"));
        assert!(cut.ends_with("[cut, 12 bytes in total]"));
    }

    #[test]
    fn usage_accepts_both_naming_styles() {
        let openai: Usage =
            serde_json::from_str(r#"{"prompt_tokens":10,"completion_tokens":5}"#).unwrap();
        let anthropic: Usage =
            serde_json::from_str(r#"{"input_tokens":10,"output_tokens":5}"#).unwrap();
        assert_eq!((openai.prompt_tokens, openai.completion_tokens), (10, 5));
        assert_eq!(
            (anthropic.prompt_tokens, anthropic.completion_tokens),
            (10, 5)
        );
    }
}
