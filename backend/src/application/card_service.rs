use std::sync::Arc;

use chrono::Utc;

use super::ports::CardRepository;
use crate::domain::card::{Card, CardFilter, CardInput, TagCount};
use crate::domain::error::{AppError, AppResult};

/// Add, edit, search and delete vocabulary cards.
///
/// `Arc<dyn CardRepository>`: a shared pointer to *some* type that implements
/// the trait. The service doesn't know (or care) that it is SQLite.
#[derive(Clone)]
pub struct CardService {
    repo: Arc<dyn CardRepository>,
}

impl CardService {
    pub fn new(repo: Arc<dyn CardRepository>) -> Self {
        Self { repo }
    }

    pub async fn list(&self, filter: &CardFilter) -> AppResult<Vec<Card>> {
        self.repo.list(filter).await
    }

    pub async fn get(&self, id: i64) -> AppResult<Card> {
        // `ok_or` turns `None` into an error; `?` returns early on error.
        self.repo.get(id).await?.ok_or(AppError::NotFound("card"))
    }

    pub async fn create(&self, input: CardInput) -> AppResult<Card> {
        let input = input.validate()?;
        self.repo.create(&input, Utc::now()).await
    }

    pub async fn update(&self, id: i64, input: CardInput) -> AppResult<Card> {
        let input = input.validate()?;
        if !self.repo.update(id, &input, Utc::now()).await? {
            return Err(AppError::NotFound("card"));
        }
        self.get(id).await
    }

    pub async fn delete(&self, id: i64) -> AppResult<()> {
        if !self.repo.delete(id).await? {
            return Err(AppError::NotFound("card"));
        }
        Ok(())
    }

    pub async fn tags(&self) -> AppResult<Vec<TagCount>> {
        self.repo.list_tags().await
    }
}
