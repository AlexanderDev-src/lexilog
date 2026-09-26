use std::sync::Arc;

use chrono::Utc;
use chrono_tz::Tz;

use super::ports::WritingRepository;
use crate::domain::error::{AppError, AppResult};
use crate::domain::writing::{
    NewPiece, Piece, PieceSummary, PieceUpdate, Version, VersionUpdate, word_count,
};

/// Writing pieces and their versions (first draft, rewrites, ...).
#[derive(Clone)]
pub struct WritingService {
    repo: Arc<dyn WritingRepository>,
    tz: Tz,
}

impl WritingService {
    pub fn new(repo: Arc<dyn WritingRepository>, tz: Tz) -> Self {
        Self { repo, tz }
    }

    pub async fn list(&self) -> AppResult<Vec<PieceSummary>> {
        self.repo.list().await
    }

    pub async fn get(&self, id: i64) -> AppResult<Piece> {
        self.repo.get(id).await?.ok_or(AppError::NotFound("piece"))
    }

    pub async fn create(&self, input: NewPiece) -> AppResult<Piece> {
        let now = Utc::now();
        // Default to today's date in the user's time zone, not UTC.
        let written_on = input
            .written_on
            .unwrap_or_else(|| now.with_timezone(&self.tz).date_naive());
        let id = self
            .repo
            .create(input.kind, input.prompt.trim(), written_on, now)
            .await?;
        self.get(id).await
    }

    pub async fn update(&self, id: i64, mut update: PieceUpdate) -> AppResult<Piece> {
        update.prompt = update.prompt.trim().to_string();
        if !self.repo.update(id, &update, Utc::now()).await? {
            return Err(AppError::NotFound("piece"));
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> AppResult<()> {
        if !self.repo.delete(id).await? {
            return Err(AppError::NotFound("piece"));
        }
        Ok(())
    }

    /// Starts a rewrite: a new version that begins as a copy of the latest one.
    pub async fn add_version(&self, piece_id: i64) -> AppResult<Version> {
        self.repo
            .add_version(piece_id, Utc::now())
            .await?
            .ok_or(AppError::NotFound("piece"))
    }

    /// Autosave target. Recounts words on the server so stored counts are
    /// always right, whatever the browser sent.
    pub async fn update_version(&self, id: i64, update: VersionUpdate) -> AppResult<Version> {
        if update.seconds_spent.is_some_and(|s| s < 0) {
            return Err(AppError::Validation(
                "seconds_spent cannot be negative".into(),
            ));
        }
        let words = word_count(&update.body);
        if !self
            .repo
            .update_version(id, &update, words, Utc::now())
            .await?
        {
            return Err(AppError::NotFound("version"));
        }
        self.repo
            .get_version(id)
            .await?
            .ok_or(AppError::NotFound("version"))
    }

    pub async fn delete_version(&self, id: i64) -> AppResult<()> {
        let version = self
            .repo
            .get_version(id)
            .await?
            .ok_or(AppError::NotFound("version"))?;
        if self.repo.count_versions(version.piece_id).await? <= 1 {
            return Err(AppError::Validation(
                "a piece needs at least one version; delete the piece instead".into(),
            ));
        }
        self.repo.delete_version(id).await?;
        Ok(())
    }
}
