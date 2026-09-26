//! The mistake log: recurring mistakes tagged on writing pieces, and how
//! often each one shows up over time.

use serde::{Deserialize, Serialize};

use super::error::{AppError, AppResult};

/// One tag on one piece, e.g. `{"tag": "missing plural -s", "count": 2}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MistakeCount {
    pub tag: String,
    pub count: i64,
}

/// A tag with its totals across all pieces (for autocomplete).
#[derive(Debug, Clone, Serialize)]
pub struct MistakeTagSummary {
    pub name: String,
    pub total: i64,
    pub pieces: i64,
}

const MAX_TAG_LEN: usize = 60;
const MAX_COUNT: i64 = 99;

/// Lower case, single spaces: "Missing  Plural -s " -> "missing plural -s".
pub fn normalize_tag(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Cleans the list the editor sends: normalises names, adds up duplicates,
/// drops rows whose count is 0 or less, and rejects nonsense.
pub fn normalize_mistakes(items: Vec<MistakeCount>) -> AppResult<Vec<MistakeCount>> {
    let mut merged: Vec<MistakeCount> = Vec::new();
    for item in items {
        let tag = normalize_tag(&item.tag);
        if tag.is_empty() || item.count <= 0 {
            continue;
        }
        if tag.chars().count() > MAX_TAG_LEN {
            return Err(AppError::Validation(format!(
                "tag names can be at most {MAX_TAG_LEN} characters"
            )));
        }
        match merged.iter_mut().find(|m| m.tag == tag) {
            Some(existing) => existing.count += item.count,
            None => merged.push(MistakeCount {
                tag,
                count: item.count,
            }),
        }
    }
    for item in &merged {
        if item.count > MAX_COUNT {
            return Err(AppError::Validation(format!(
                "a count can be at most {MAX_COUNT}"
            )));
        }
    }
    Ok(merged)
}

// ---- trend ---------------------------------------------------------------

/// How to group pieces over time. In the query string: `week` or `month`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendBucket {
    Week,
    #[default]
    Month,
}

/// How much was written in one period. Words are counted on each piece's
/// FIRST version, where the mistakes were made.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PeriodWords {
    /// "2026-09" for months, "2026-W38" for weeks.
    pub period: String,
    pub pieces: i64,
    pub words: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TrendPoint {
    pub period: String,
    pub count: i64,
    /// Mistakes per 1000 words, so a busy month doesn't look worse just
    /// because more was written.
    pub per_1000_words: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    /// The latest period is clearly lower than before.
    Fading,
    /// The latest period is clearly higher than before.
    Rising,
    Steady,
    /// Only seen in the latest period.
    New,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TagTrend {
    pub tag: String,
    pub total: i64,
    pub points: Vec<TrendPoint>,
    pub direction: Direction,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MistakeTrend {
    pub bucket: TrendBucket,
    pub periods: Vec<PeriodWords>,
    pub tags: Vec<TagTrend>,
}

/// A raw row from the database: `count` mistakes of `tag` in `period`.
#[derive(Debug, Clone)]
pub struct PeriodCount {
    pub period: String,
    pub tag: String,
    pub count: i64,
}

/// Builds the per-tag series. `periods` must be sorted oldest first.
pub fn build_trend(
    bucket: TrendBucket,
    periods: Vec<PeriodWords>,
    counts: Vec<PeriodCount>,
) -> MistakeTrend {
    let mut tag_names: Vec<String> = Vec::new();
    for row in &counts {
        if !tag_names.contains(&row.tag) {
            tag_names.push(row.tag.clone());
        }
    }

    let mut tags: Vec<TagTrend> = tag_names
        .into_iter()
        .map(|tag| {
            let points: Vec<TrendPoint> = periods
                .iter()
                .map(|p| {
                    let count = counts
                        .iter()
                        .filter(|c| c.tag == tag && c.period == p.period)
                        .map(|c| c.count)
                        .sum();
                    let per_1000_words = if p.words > 0 {
                        (count as f64 * 1000.0 / p.words as f64 * 10.0).round() / 10.0
                    } else {
                        0.0
                    };
                    TrendPoint {
                        period: p.period.clone(),
                        count,
                        per_1000_words,
                    }
                })
                .collect();
            let total = points.iter().map(|p| p.count).sum();
            let direction = direction(&points, &periods);
            TagTrend {
                tag,
                total,
                points,
                direction,
            }
        })
        .collect();

    // Most frequent mistakes first.
    tags.sort_by(|a, b| b.total.cmp(&a.total).then(a.tag.cmp(&b.tag)));

    MistakeTrend {
        bucket,
        periods,
        tags,
    }
}

/// Compares the latest period that has writing with the average of the
/// earlier ones. 30% either way counts as a real change.
fn direction(points: &[TrendPoint], periods: &[PeriodWords]) -> Direction {
    let rates: Vec<f64> = points
        .iter()
        .zip(periods)
        .filter(|(_, period)| period.words > 0)
        .map(|(point, _)| point.per_1000_words)
        .collect();

    // `split_last` gives (last item, everything before it).
    let Some((&latest, earlier)) = rates.split_last() else {
        return Direction::Steady;
    };
    if earlier.is_empty() {
        return Direction::Steady;
    }
    let earlier_avg = earlier.iter().sum::<f64>() / earlier.len() as f64;
    if earlier_avg == 0.0 {
        return if latest > 0.0 {
            Direction::New
        } else {
            Direction::Steady
        };
    }
    if latest < earlier_avg * 0.7 {
        Direction::Fading
    } else if latest > earlier_avg * 1.3 {
        Direction::Rising
    } else {
        Direction::Steady
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(tag: &str, n: i64) -> MistakeCount {
        MistakeCount {
            tag: tag.into(),
            count: n,
        }
    }

    #[test]
    fn normalize_merges_and_drops() {
        let cleaned = normalize_mistakes(vec![
            count("Missing  plural -s", 1),
            count("missing plural -s ", 2),
            count("articles", 0),
            count("   ", 3),
        ])
        .unwrap();
        assert_eq!(cleaned, vec![count("missing plural -s", 3)]);
    }

    #[test]
    fn normalize_rejects_huge_counts() {
        assert!(normalize_mistakes(vec![count("articles", 500)]).is_err());
    }

    fn period(name: &str, words: i64) -> PeriodWords {
        PeriodWords {
            period: name.into(),
            pieces: 1,
            words,
        }
    }

    fn row(period: &str, tag: &str, count: i64) -> PeriodCount {
        PeriodCount {
            period: period.into(),
            tag: tag.into(),
            count,
        }
    }

    #[test]
    fn trend_normalises_by_words_and_spots_fading() {
        let trend = build_trend(
            TrendBucket::Month,
            vec![
                period("2026-07", 500),
                period("2026-08", 1000),
                period("2026-09", 1000),
            ],
            vec![
                row("2026-07", "plural -s", 5), // 10 per 1000 words
                row("2026-08", "plural -s", 8), // 8 per 1000
                row("2026-09", "plural -s", 2), // 2 per 1000: fading
                row("2026-09", "articles", 3),  // only in the latest month
            ],
        );
        let plural = &trend.tags[0];
        assert_eq!(plural.tag, "plural -s");
        assert_eq!(plural.total, 15);
        assert_eq!(plural.points[0].per_1000_words, 10.0);
        assert_eq!(plural.direction, Direction::Fading);
        assert_eq!(trend.tags[1].direction, Direction::New);
    }
}
