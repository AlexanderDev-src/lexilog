use std::sync::Arc;

use super::ports::MistakeRepository;
use crate::domain::error::{AppError, AppResult};
use crate::domain::mistakes::{
    MistakeCount, MistakeTagSummary, MistakeTrend, TrendBucket, build_trend, normalize_mistakes,
};

/// The mistake log: tags on pieces, and the trend over time.
#[derive(Clone)]
pub struct MistakeService {
    repo: Arc<dyn MistakeRepository>,
}

impl MistakeService {
    pub fn new(repo: Arc<dyn MistakeRepository>) -> Self {
        Self { repo }
    }

    pub async fn for_piece(&self, piece_id: i64) -> AppResult<Vec<MistakeCount>> {
        self.repo.for_piece(piece_id).await
    }

    /// Replaces the piece's mistakes with `items` (after cleaning them).
    pub async fn replace(
        &self,
        piece_id: i64,
        items: Vec<MistakeCount>,
    ) -> AppResult<Vec<MistakeCount>> {
        let items = normalize_mistakes(items)?;
        if !self.repo.replace_for_piece(piece_id, &items).await? {
            return Err(AppError::NotFound("piece"));
        }
        self.repo.for_piece(piece_id).await
    }

    pub async fn tags(&self) -> AppResult<Vec<MistakeTagSummary>> {
        self.repo.tags().await
    }

    pub async fn trend(&self, bucket: TrendBucket) -> AppResult<MistakeTrend> {
        let (periods, counts) = self.repo.trend_data(bucket).await?;
        Ok(build_trend(bucket, periods, counts))
    }
}
