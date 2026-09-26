use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, SqlitePool};

use super::database::timestamp;
use crate::application::ports::PracticeRepository;
use crate::domain::error::AppResult;
use crate::domain::practice::{NewPractice, PracticeSession, Skill};

pub struct SqlitePracticeRepository {
    pool: SqlitePool,
}

impl SqlitePracticeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[derive(FromRow)]
struct PracticeRow {
    id: i64,
    practiced_on: NaiveDate,
    skill: String,
    minutes: i64,
    note: String,
    created_at: DateTime<Utc>,
}

impl PracticeRow {
    fn into_session(self) -> AppResult<PracticeSession> {
        Ok(PracticeSession {
            id: self.id,
            practiced_on: self.practiced_on,
            skill: Skill::parse(&self.skill)?,
            minutes: self.minutes,
            note: self.note,
            created_at: self.created_at,
        })
    }
}

#[async_trait]
impl PracticeRepository for SqlitePracticeRepository {
    async fn list(&self, limit: i64) -> AppResult<Vec<PracticeSession>> {
        let rows = sqlx::query_as::<_, PracticeRow>(
            "SELECT id, practiced_on, skill, minutes, note, created_at
               FROM practice_sessions
              ORDER BY practiced_on DESC, id DESC
              LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(PracticeRow::into_session).collect()
    }

    async fn create(&self, practice: &NewPractice, now: DateTime<Utc>) -> AppResult<i64> {
        let id = sqlx::query(
            "INSERT INTO practice_sessions (practiced_on, skill, minutes, note, created_at)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(practice.practiced_on)
        .bind(practice.skill.as_str())
        .bind(practice.minutes)
        .bind(&practice.note)
        .bind(timestamp(now))
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    async fn get(&self, id: i64) -> AppResult<Option<PracticeSession>> {
        let row = sqlx::query_as::<_, PracticeRow>(
            "SELECT id, practiced_on, skill, minutes, note, created_at
               FROM practice_sessions WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(PracticeRow::into_session).transpose()
    }

    async fn update(&self, id: i64, practice: &NewPractice) -> AppResult<bool> {
        let result = sqlx::query(
            "UPDATE practice_sessions SET practiced_on = ?, skill = ?, minutes = ?, note = ?
             WHERE id = ?",
        )
        .bind(practice.practiced_on)
        .bind(practice.skill.as_str())
        .bind(practice.minutes)
        .bind(&practice.note)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    async fn delete(&self, id: i64) -> AppResult<bool> {
        let result = sqlx::query("DELETE FROM practice_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
