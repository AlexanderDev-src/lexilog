use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{AssertSqlSafe, FromRow, SqliteConnection, SqlitePool};

use super::database::timestamp;
use crate::application::ports::CardRepository;
use crate::domain::card::{Card, CardFilter, CardInput, DueBreakdown, TagCount};
use crate::domain::error::{AppError, AppResult};
use crate::domain::review::NewReviewLog;

pub struct SqliteCardRepository {
    pool: SqlitePool,
}

impl SqliteCardRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

/// One row as SQLite returns it. `#[derive(FromRow)]` maps columns to
/// fields by name. It stays private: the rest of the app sees `Card`.
#[derive(FromRow)]
struct CardRow {
    id: i64,
    word: String,
    meaning: String,
    example: String,
    source: String,
    /// JSON array text such as `["education","environment"]`.
    tags: String,
    stability: Option<f64>,
    difficulty: Option<f64>,
    due: DateTime<Utc>,
    last_review: Option<DateTime<Utc>>,
    reps: i64,
    lapses: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CardRow {
    fn into_card(self) -> AppResult<Card> {
        let tags = serde_json::from_str(&self.tags)
            .map_err(|err| AppError::Internal(format!("bad tags json: {err}")))?;
        Ok(Card {
            id: self.id,
            word: self.word,
            meaning: self.meaning,
            example: self.example,
            source: self.source,
            tags,
            stability: self.stability,
            difficulty: self.difficulty,
            due: self.due,
            last_review: self.last_review,
            reps: self.reps,
            lapses: self.lapses,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

/// Every card query starts with this. The sub-select collects the card's tag
/// names into one JSON array, so we get cards and tags in a single query.
///
/// Queries built with `format!("{SELECT_CARD} WHERE ...")` must be wrapped in
/// `AssertSqlSafe(...)`: sqlx refuses non-literal SQL by default, to stop
/// user input being pasted into SQL. Ours only joins constant text; every
/// user value goes through `.bind()`.
const SELECT_CARD: &str = "
    SELECT c.id, c.word, c.meaning, c.example, c.source,
           (SELECT json_group_array(t.name ORDER BY t.name)
              FROM card_tags ct JOIN tags t ON t.id = ct.tag_id
             WHERE ct.card_id = c.id) AS tags,
           c.stability, c.difficulty, c.due, c.last_review, c.reps, c.lapses,
           c.created_at, c.updated_at
      FROM cards c";

fn into_cards(rows: Vec<CardRow>) -> AppResult<Vec<Card>> {
    rows.into_iter().map(CardRow::into_card).collect()
}

/// Replaces a card's tags. Takes a plain connection so it can run inside
/// the caller's transaction.
async fn set_tags(conn: &mut SqliteConnection, card_id: i64, tags: &[String]) -> AppResult<()> {
    sqlx::query("DELETE FROM card_tags WHERE card_id = ?")
        .bind(card_id)
        .execute(&mut *conn)
        .await?;
    for tag in tags {
        sqlx::query("INSERT INTO tags (name) VALUES (?) ON CONFLICT (name) DO NOTHING")
            .bind(tag)
            .execute(&mut *conn)
            .await?;
        sqlx::query(
            "INSERT INTO card_tags (card_id, tag_id) SELECT ?, id FROM tags WHERE name = ?",
        )
        .bind(card_id)
        .bind(tag)
        .execute(&mut *conn)
        .await?;
    }
    Ok(())
}

/// Tags no card uses any more would clutter the tag list, so drop them.
async fn delete_unused_tags(conn: &mut SqliteConnection) -> AppResult<()> {
    sqlx::query("DELETE FROM tags WHERE id NOT IN (SELECT tag_id FROM card_tags)")
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Wraps a search term for LIKE and escapes `%` and `_`, which LIKE would
/// otherwise treat as wildcards.
fn like_pattern(term: &str) -> String {
    let escaped = term
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    format!("%{escaped}%")
}

#[async_trait]
impl CardRepository for SqliteCardRepository {
    async fn list(&self, filter: &CardFilter) -> AppResult<Vec<Card>> {
        // `?1 IS NULL OR ...` makes each filter optional: binding NULL turns it off.
        let sql = format!(
            "{SELECT_CARD}
             WHERE (?1 IS NULL OR c.word LIKE ?1 ESCAPE '\\'
                              OR c.meaning LIKE ?1 ESCAPE '\\'
                              OR c.example LIKE ?1 ESCAPE '\\')
               AND (?2 IS NULL OR EXISTS (
                       SELECT 1 FROM card_tags ct JOIN tags t ON t.id = ct.tag_id
                        WHERE ct.card_id = c.id AND t.name = ?2))
             ORDER BY c.created_at DESC, c.id DESC
             LIMIT ?3 OFFSET ?4"
        );
        let q = filter
            .q
            .as_deref()
            .map(str::trim)
            .filter(|q| !q.is_empty())
            .map(like_pattern);
        let tag = filter
            .tag
            .as_deref()
            .map(|t| t.trim().to_lowercase())
            .filter(|t| !t.is_empty());

        let rows = sqlx::query_as::<_, CardRow>(AssertSqlSafe(sql))
            .bind(q)
            .bind(tag)
            .bind(filter.limit())
            .bind(filter.offset())
            .fetch_all(&self.pool)
            .await?;
        into_cards(rows)
    }

    async fn get(&self, id: i64) -> AppResult<Option<Card>> {
        let sql = format!("{SELECT_CARD} WHERE c.id = ?");
        let row = sqlx::query_as::<_, CardRow>(AssertSqlSafe(sql))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        // Option<CardRow> -> Option<AppResult<Card>> -> AppResult<Option<Card>>
        row.map(CardRow::into_card).transpose()
    }

    async fn create(&self, input: &CardInput, now: DateTime<Utc>) -> AppResult<Card> {
        let now = timestamp(now);
        // A transaction: either the card *and* its tags are saved, or nothing is.
        let mut tx = self.pool.begin().await?;

        // `&mut *tx` borrows the connection inside the transaction.
        let id = sqlx::query(
            "INSERT INTO cards (word, meaning, example, source, due, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&input.word)
        .bind(&input.meaning)
        .bind(&input.example)
        .bind(&input.source)
        .bind(&now) // a new card is due immediately
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        set_tags(&mut tx, id, &input.tags).await?;
        tx.commit().await?;

        self.get(id)
            .await?
            .ok_or_else(|| AppError::Internal("card vanished after insert".into()))
    }

    async fn update(&self, id: i64, input: &CardInput, now: DateTime<Utc>) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;
        let result = sqlx::query(
            "UPDATE cards SET word = ?, meaning = ?, example = ?, source = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(&input.word)
        .bind(&input.meaning)
        .bind(&input.example)
        .bind(&input.source)
        .bind(timestamp(now))
        .bind(id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false); // dropping `tx` without commit rolls it back
        }
        set_tags(&mut tx, id, &input.tags).await?;
        delete_unused_tags(&mut tx).await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn delete(&self, id: i64) -> AppResult<bool> {
        let mut tx = self.pool.begin().await?;
        // ON DELETE CASCADE removes its card_tags and review_logs too.
        let result = sqlx::query("DELETE FROM cards WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        delete_unused_tags(&mut tx).await?;
        tx.commit().await?;
        Ok(result.rows_affected() > 0)
    }

    async fn list_tags(&self) -> AppResult<Vec<TagCount>> {
        // query_as into a tuple: (name, card_count) per row.
        let rows = sqlx::query_as::<_, (String, i64)>(
            "SELECT t.name, COUNT(ct.card_id)
               FROM tags t LEFT JOIN card_tags ct ON ct.tag_id = t.id
              GROUP BY t.id
              ORDER BY t.name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(name, card_count)| TagCount { name, card_count })
            .collect())
    }

    async fn due_before(&self, cutoff: DateTime<Utc>, limit: i64) -> AppResult<Vec<Card>> {
        let sql = format!("{SELECT_CARD} WHERE c.due < ? ORDER BY c.due, c.id LIMIT ?");
        let rows = sqlx::query_as::<_, CardRow>(AssertSqlSafe(sql))
            .bind(timestamp(cutoff))
            .bind(limit)
            .fetch_all(&self.pool)
            .await?;
        into_cards(rows)
    }

    async fn count_due_before(&self, cutoff: DateTime<Utc>) -> AppResult<i64> {
        let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM cards WHERE due < ?")
            .bind(timestamp(cutoff))
            .fetch_one(&self.pool)
            .await?;
        Ok(count)
    }

    async fn due_breakdown(
        &self,
        cutoff: DateTime<Utc>,
        today_start: DateTime<Utc>,
    ) -> AppResult<DueBreakdown> {
        // In SQLite a comparison is 0 or 1, so SUM(condition) counts matches.
        let (new, again, review) = sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT COALESCE(SUM(stability IS NULL), 0),
                    COALESCE(SUM(stability IS NOT NULL AND last_review >= ?2), 0),
                    COALESCE(SUM(stability IS NOT NULL AND last_review < ?2), 0)
               FROM cards
              WHERE due < ?1",
        )
        .bind(timestamp(cutoff))
        .bind(timestamp(today_start))
        .fetch_one(&self.pool)
        .await?;
        Ok(DueBreakdown { new, review, again })
    }

    async fn save_review(&self, card: &Card, log: &NewReviewLog) -> AppResult<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            "UPDATE cards
                SET stability = ?, difficulty = ?, due = ?, last_review = ?, reps = ?, lapses = ?
              WHERE id = ?",
        )
        .bind(card.stability)
        .bind(card.difficulty)
        .bind(timestamp(card.due))
        .bind(card.last_review.map(timestamp))
        .bind(card.reps)
        .bind(card.lapses)
        .bind(card.id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "INSERT INTO review_logs
                (card_id, rating, reviewed_at, elapsed_days,
                 stability_before, difficulty_before, stability_after, difficulty_after,
                 scheduled_days, duration_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(log.card_id)
        .bind(u8::from(log.rating))
        .bind(timestamp(log.reviewed_at))
        .bind(log.elapsed_days)
        .bind(log.before.map(|m| m.stability))
        .bind(log.before.map(|m| m.difficulty))
        .bind(log.after.stability)
        .bind(log.after.difficulty)
        .bind(log.scheduled_days)
        .bind(log.duration_ms)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}
