use chrono_tz::Tz;

use crate::application::ai_feedback_service::ModelLimit;

/// Settings read from environment variables, with defaults for local dev.
pub struct Config {
    /// Address to listen on. 0.0.0.0 = reachable from the home network.
    pub bind: String,
    /// SQLite file. Its folder is created if missing.
    pub database_path: String,
    /// Built frontend (`npm run build` output).
    pub static_dir: String,
    /// Decides where "today" starts and ends.
    pub tz: Tz,
    pub desired_retention: f32,
    pub ai: AiConfig,
}

/// AI writing feedback through an OpenAI-compatible gateway.
pub struct AiConfig {
    pub base_url: String,
    /// `None` = AI feedback switched off.
    pub api_key: Option<String>,
    /// Models the app may use, each with its daily token limit.
    pub models: Vec<ModelLimit>,
    pub default_model: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let tz_name = env_or("APP_TZ", "Asia/Bangkok");
        let tz: Tz = tz_name
            .parse()
            .map_err(|_| format!("APP_TZ: unknown time zone '{tz_name}'"))?;

        let desired_retention: f32 = env_or("DESIRED_RETENTION", "0.9")
            .parse()
            .ok()
            .filter(|r| (0.7..=0.99).contains(r))
            .ok_or("DESIRED_RETENTION must be a number from 0.70 to 0.99")?;

        let models = parse_models(&env_or("AI_MODELS", "claude-sonnet-5:200000"))?;
        let default_model = match std::env::var("AI_DEFAULT_MODEL") {
            Ok(name) if !name.trim().is_empty() => {
                let name = name.trim().to_string();
                if !models.iter().any(|m| m.id == name) {
                    return Err(format!(
                        "AI_DEFAULT_MODEL '{name}' is not listed in AI_MODELS"
                    ));
                }
                Some(name)
            }
            _ => models.first().map(|m| m.id.clone()),
        };
        let ai = AiConfig {
            base_url: env_or("AI_BASE_URL", "https://gen.ai.kku.ac.th/api/v1"),
            api_key: std::env::var("AI_API_KEY")
                .ok()
                .map(|key| key.trim().to_string())
                .filter(|key| !key.is_empty()),
            models,
            default_model,
        };

        Ok(Config {
            bind: env_or("APP_BIND", "0.0.0.0:1111"),
            database_path: env_or("DATABASE_PATH", "data/ielts.db"),
            static_dir: env_or("STATIC_DIR", "../frontend/dist"),
            tz,
            desired_retention,
            ai,
        })
    }
}

/// Parses `AI_MODELS`: "claude-sonnet-5:200000,deepseek-v4-pro:1000000".
/// Model names contain `-` and `.`, so the limit is whatever follows the LAST `:`.
fn parse_models(text: &str) -> Result<Vec<ModelLimit>, String> {
    text.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            let (id, limit) = item.rsplit_once(':').ok_or_else(|| {
                format!("AI_MODELS: '{item}' should look like model:daily_tokens")
            })?;
            let daily_limit = limit
                .trim()
                .parse::<i64>()
                .map_err(|_| format!("AI_MODELS: '{limit}' is not a whole number"))?;
            Ok(ModelLimit {
                id: id.trim().to_string(),
                daily_limit,
            })
        })
        .collect()
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_model_list() {
        let models = parse_models("claude-sonnet-5:200000, gemini-3.8-flash:350000").unwrap();
        assert_eq!(models.len(), 2);
        assert_eq!(models[1].id, "gemini-3.8-flash");
        assert_eq!(models[1].daily_limit, 350_000);
        assert!(parse_models("claude-sonnet-5").is_err());
        assert!(parse_models("claude-sonnet-5:lots").is_err());
    }
}
