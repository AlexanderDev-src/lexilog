use std::sync::Arc;

use chrono::Utc;
use chrono_tz::Tz;

use super::ports::PracticeRepository;
use crate::domain::calendar::today;
use crate::domain::error::{AppError, AppResult};
use crate::domain::practice::{PracticeInput, PracticeSession};

/// The practice log: listening, reading, speaking and other practice.
#[derive(Clone)]
pub struct PracticeService {
    repo: Arc<dyn PracticeRepository>,
    tz: Tz,
}

impl PracticeService {
    pub fn new(repo: Arc<dyn PracticeRepository>, tz: Tz) -> Self {
        Self { repo, tz }
    }

    pub async fn list(&self, limit: i64) -> AppResult<Vec<PracticeSession>> {
        self.repo.list(limit.clamp(1, 500)).await
    }

    pub async fn create(&self, input: PracticeInput) -> AppResult<PracticeSession> {
        let now = Utc::now();
        let practice = input.validate(today(now, self.tz))?;
        let id = self.repo.create(&practice, now).await?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, input: PracticeInput) -> AppResult<PracticeSession> {
        let practice = input.validate(today(Utc::now(), self.tz))?;
        if !self.repo.update(id, &practice).await? {
            return Err(AppError::NotFound("practice session"));
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> AppResult<()> {
        if !self.repo.delete(id).await? {
            return Err(AppError::NotFound("practice session"));
        }
        Ok(())
    }

    async fn get(&self, id: i64) -> AppResult<PracticeSession> {
        self.repo
            .get(id)
            .await?
            .ok_or(AppError::NotFound("practice session"))
    }
}
