use valet::config::Config;
use valet::embeddings::create_provider;
use valet::embeddings::reindex::{reindex_all, reset_memory_source};

const USAGE: &str = "\
valet-reindex — reconstruct the episodic-memory index

USAGE:
    valet-reindex [--reset]

Without arguments, regenerate the embedding of every row in `memory` and upsert
it into the `vec_memory` index (requires EMBEDDING_PROVIDER and EMBEDDING_MODEL).

OPTIONS:
    --reset     DESTRUCTIVE: reset the source the index is rebuilt from, then
                exit without re-embedding. It clears the indexing state of
                EVERY message and DELETES every card in `memory` and every
                vector in `vec_memory`:

                    UPDATE messages SET is_indexed = 0, summary_ref = NULL;
                    DELETE FROM memory;
                    DELETE FROM vec_memory;

                After this, start the app so the EpisodicMemoryWorker re-archives
                from the original messages.
    -h, --help  Print this help and exit.
";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!("{USAGE}");
        return Ok(());
    }

    if let Some(unknown) = args.iter().find(|a| a.as_str() != "--reset") {
        eprintln!("error: unknown argument '{unknown}'\n");
        eprint!("{USAGE}");
        std::process::exit(2);
    }

    let reset = args.iter().any(|a| a == "--reset");

    let config = Config::from_env();
    let pool = valet::db::init_db(&config.database_url).await?;

    if reset {
        eprintln!(
            "WARNING: destructive reset of the episodic-memory source.\n\
             This will, on database '{}':\n\
             \x20 - set is_indexed = 0 and summary_ref = NULL on EVERY row of `messages`\n\
             \x20 - DELETE every row of `memory`\n\
             \x20 - DELETE every row of `vec_memory`\n\
             After the reset, start the app so the EpisodicMemoryWorker re-archives \
             from the original messages.",
            config.database_url
        );

        let report = reset_memory_source(&pool).await?;
        println!(
            "Reset complete: messages_reset={} memories_deleted={} vectors_deleted={}",
            report.messages_reset, report.memories_deleted, report.vectors_deleted
        );
        return Ok(());
    }

    let provider = create_provider(&config)
        .ok_or("embeddings not configured: set EMBEDDING_PROVIDER and EMBEDDING_MODEL")?;
    let report = reindex_all(&pool, provider.as_ref(), config.embedding_dimension).await?;
    println!(
        "Reindex complete: total={} updated={} failed={}",
        report.total, report.updated, report.failed
    );
    Ok(())
}
