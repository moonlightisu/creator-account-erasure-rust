use crate::{CreatorRecords, InfraiClient, InfraiError, RecordCounts};
use thiserror::Error;

#[derive(Debug, PartialEq, Eq)]
pub struct DeletionReceipt {
    pub creator_id: String,
    pub revoked_sessions: usize,
    pub erased_records: RecordCounts,
    pub credential_revoked: bool,
}

#[derive(Debug, Error)]
pub enum DeletionError {
    #[error(transparent)]
    Infrai(#[from] InfraiError),
    #[error("creator records remained after erasure")]
    RecordsRemain,
}

#[derive(Clone, Debug)]
pub struct DeletionService {
    infrai: InfraiClient,
}

impl DeletionService {
    pub fn new(infrai: InfraiClient) -> Self {
        Self { infrai }
    }

    pub async fn delete_creator(
        &self,
        creator_id: &str,
        credential_id: &str,
        records: &mut CreatorRecords,
    ) -> Result<DeletionReceipt, DeletionError> {
        let sessions = self.infrai.list_sessions(creator_id).await?;
        for session_id in &sessions {
            self.infrai.revoke_session(session_id).await?;
        }

        let erased_records = records.erase_all();
        if !records.is_empty() {
            return Err(DeletionError::RecordsRemain);
        }

        // Revoke last so the same control-plane credential can finish the account cleanup.
        self.infrai.revoke_credential(credential_id).await?;

        Ok(DeletionReceipt {
            creator_id: creator_id.to_owned(),
            revoked_sessions: sessions.len(),
            erased_records,
            credential_revoked: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erasure_covers_every_creator_record_class() {
        let mut records = CreatorRecords::new(RecordCounts {
            digital_asset_deliveries: 3,
            subscriber_updates: 8,
            content_processing_jobs: 2,
        });

        let erased = records.erase_all();

        assert_eq!(erased.total(), 13);
        assert!(records.is_empty());
        assert_eq!(erased.digital_asset_deliveries, 3);
        assert_eq!(erased.subscriber_updates, 8);
        assert_eq!(erased.content_processing_jobs, 2);
    }
}
