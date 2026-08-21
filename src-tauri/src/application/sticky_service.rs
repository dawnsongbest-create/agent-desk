use std::{collections::HashSet, sync::Arc};

use thiserror::Error;

use crate::{
    application::ports::sticky_repository::{StickyRepository, StickyRepositoryError},
    domain::sticky::{
        normalize_text, validate_due_date, validate_quote, NewStickyCard, StickyCard,
        StickyCardKind, StickyProfile, StickyValidationError,
    },
};

#[derive(Debug, Error)]
pub enum StickyServiceError {
    #[error(transparent)]
    Validation(#[from] StickyValidationError),
    #[error(transparent)]
    Repository(#[from] StickyRepositoryError),
}

#[derive(Clone)]
pub struct StickyService {
    repository: Arc<dyn StickyRepository>,
}

impl StickyService {
    pub fn new(repository: Arc<dyn StickyRepository>) -> Self {
        Self { repository }
    }

    pub async fn list(&self) -> Result<Vec<StickyCard>, StickyServiceError> {
        Ok(self.repository.list().await?)
    }

    pub async fn create(
        &self,
        kind: StickyCardKind,
        text: String,
        due_date: Option<String>,
    ) -> Result<StickyCard, StickyServiceError> {
        let card = Self::prepare_create(kind, text, due_date)?;
        Ok(self.repository.create(&card).await?)
    }

    pub fn prepare_create(
        kind: StickyCardKind,
        text: String,
        due_date: Option<String>,
    ) -> Result<NewStickyCard, StickyServiceError> {
        let due_date = validate_due_date(due_date)?;
        if kind == StickyCardKind::Note && due_date.is_some() {
            return Err(StickyValidationError::NoteDueDate.into());
        }
        Ok(NewStickyCard {
            kind,
            text: normalize_text(
                text,
                if kind == StickyCardKind::Note {
                    100_000
                } else {
                    4_000
                },
            )?,
            due_date,
        })
    }

    pub async fn update_text(
        &self,
        id: &str,
        text: String,
    ) -> Result<StickyCard, StickyServiceError> {
        let text = normalize_text(text, 100_000)?;
        Ok(self.repository.update_text(id, &text).await?)
    }

    pub async fn set_task_completed(
        &self,
        id: &str,
        completed: bool,
    ) -> Result<StickyCard, StickyServiceError> {
        Ok(self.repository.set_task_completed(id, completed).await?)
    }

    pub async fn set_task_due_date(
        &self,
        id: &str,
        due_date: Option<String>,
    ) -> Result<StickyCard, StickyServiceError> {
        let due_date = validate_due_date(due_date)?;
        Ok(self
            .repository
            .set_task_due_date(id, due_date.as_deref())
            .await?)
    }

    pub async fn delete(&self, id: &str) -> Result<(), StickyServiceError> {
        Ok(self.repository.delete(id).await?)
    }

    pub async fn reorder(
        &self,
        ordered_ids: Vec<String>,
    ) -> Result<Vec<StickyCard>, StickyServiceError> {
        let unique: HashSet<_> = ordered_ids.iter().collect();
        if unique.len() != ordered_ids.len() {
            return Err(StickyValidationError::DuplicatePlacement.into());
        }
        Ok(self.repository.reorder(&ordered_ids).await?)
    }

    pub async fn get_profile(&self) -> Result<StickyProfile, StickyServiceError> {
        Ok(self.repository.get_profile().await?)
    }

    pub async fn update_quote(
        &self,
        quote_text: String,
    ) -> Result<StickyProfile, StickyServiceError> {
        let quote_text = validate_quote(quote_text)?;
        Ok(self.repository.update_quote(&quote_text).await?)
    }

    pub async fn record_text(&self, id: &str) -> Result<String, StickyServiceError> {
        Ok(self.repository.get_record_text(id).await?)
    }
}
