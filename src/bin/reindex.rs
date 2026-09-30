use valet::config::Config;
use valet::embeddings::{create_provider, reindex::reindex_all};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let provider = create_provider(&config)
        .ok_or("embeddings not configured: set EMBEDDING_PROVIDER and EMBEDDING_MODEL")?;
    let pool = valet::db::init_db(&config.database_url).await?;
    let report = reindex_all(&pool, provider.as_ref(), config.embedding_dimension).await?;
    println!(
        "Reindex complete: total={} updated={} failed={}",
        report.total, report.updated, report.failed
    );
    Ok(())
}
