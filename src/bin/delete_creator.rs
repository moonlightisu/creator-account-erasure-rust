use creator_account_erasure::{CreatorRecords, DeletionService, InfraiClient, RecordCounts};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let creator_id = required_env("CREATOR_ID")?;
    let credential_id = required_env("CREATOR_CREDENTIAL_ID")?;
    let client = InfraiClient::from_env()?;
    let service = DeletionService::new(client);

    // Replace these counts with records loaded from the product database.
    let mut records = CreatorRecords::new(RecordCounts {
        digital_asset_deliveries: env_count("DELIVERY_RECORDS")?,
        subscriber_updates: env_count("SUBSCRIBER_UPDATE_RECORDS")?,
        content_processing_jobs: env_count("CONTENT_JOB_RECORDS")?,
    });

    let receipt = service
        .delete_creator(&creator_id, &credential_id, &mut records)
        .await?;
    println!(
        "deleted creator={} sessions={} records={} credential_revoked={}",
        receipt.creator_id,
        receipt.revoked_sessions,
        receipt.erased_records.total(),
        receipt.credential_revoked
    );
    Ok(())
}

fn required_env(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    std::env::var(name).map_err(|_| format!("{name} is required").into())
}

fn env_count(name: &str) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(std::env::var(name)
        .unwrap_or_else(|_| "0".to_owned())
        .parse()?)
}
