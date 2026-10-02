//! Registration and start-up verification of the `sqlite-vec` extension.
//!
//! `sqlite-vec` exposes the `vec0` virtual table through the C entry point
//! `sqlite3_vec_init`, which must be registered with SQLite *before* any
//! connection is opened. The standard mechanism is
//! [`sqlite3_auto_extension`][auto], which makes SQLite call the initialiser on
//! **every** connection it opens after registration — including the ones
//! `sqlx` creates lazily inside its pool.
//!
//! [auto]: https://www.sqlite.org/c3ref/auto_extension.html
//!
//! Two properties of that mechanism shape this module:
//!
//! * It is **global to the process** and **cannot be undone**, so it is fenced
//!   by a [`Once`] to keep the initialiser from running more times than
//!   necessary.
//! * It only affects connections opened *after* the call, so registration must
//!   happen before the pool is created.

use std::sync::Once;

use sqlx::SqlitePool;

/// The exact function-pointer type that `sqlite3_auto_extension` accepts.
///
/// `sqlite_vec::sqlite3_vec_init` is declared as `fn()` by the crate because its
/// real C signature is type-erased from Rust's point of view; this alias lets us
/// name the concrete target of the `transmute` below.
type AutoExtensionEntry = unsafe extern "C" fn(
    db: *mut libsqlite3_sys::sqlite3,
    pz_err_msg: *mut *mut std::os::raw::c_char,
    api: *const libsqlite3_sys::sqlite3_api_routines,
) -> std::os::raw::c_int;

/// Guards the one-time registration of the extension.
static REGISTER_EXTENSION: Once = Once::new();

/// Register the extension at process load time, before `main` (and therefore
/// before any connection is opened).
///
/// `sqlite3_auto_extension` only affects connections opened *after* it runs, so
/// registering it from `init_db` is enough for production but not for the test
/// helpers that open a pool and then run the migrator directly. Running it from
/// an ELF `.init_array` constructor makes the registration happen before any
/// code — and thus any connection — in the process, covering both.
///
/// `register_vec_extension` is idempotent, so this is a no-op if something else
/// already registered it.
#[cfg(target_os = "linux")]
#[used]
#[link_section = ".init_array"]
static REGISTER_VEC_EXTENSION_ON_LOAD: extern "C" fn() = {
    extern "C" fn init() {
        register_vec_extension();
    }
    init
};

/// Register the `sqlite-vec` extension for every connection this process opens.
///
/// Safe to call from anywhere and any number of times: the first call performs
/// the registration, the rest are no-ops. Must be invoked **before** the pool is
/// created, because auto-extensions are applied to connections as they are
/// opened.
pub fn register_vec_extension() {
    REGISTER_EXTENSION.call_once(|| {
        // SAFETY: `sqlite3_vec_init` is the C entry point exported by the
        // statically linked `sqlite_vec0` library, with the canonical SQLite
        // extension signature
        // `int (*)(sqlite3*, char**, const sqlite3_api_routines*)`. The Rust
        // declaration exposed by the `sqlite-vec` crate is type-erased to
        // `fn()`, so its address is transmuted back to that exact C
        // function-pointer type; both share the `extern "C"` ABI and the symbol
        // lives for the whole process, so the pointer is valid forever. The
        // call is process-global and has no failure mode to report (SQLite
        // returns an int we do not need), and `Once` guarantees it runs exactly
        // once even if multiple threads race to start the app or the test.
        unsafe {
            libsqlite3_sys::sqlite3_auto_extension(Some(std::mem::transmute::<
                *const (),
                AutoExtensionEntry,
            >(
                sqlite_vec::sqlite3_vec_init as *const (),
            )));
        }
    });
}

/// Error returned when the `sqlite-vec` extension is not usable at start-up.
#[derive(Debug, thiserror::Error)]
pub enum VecExtensionError {
    /// The probe query (`SELECT vec_version()`) did not succeed, which means the
    /// extension is not registered on the connection it ran on.
    #[error(
        "sqlite-vec extension is not available: `SELECT vec_version()` failed. \
         The extension is not registered (missing `sqlite3_auto_extension(\
         sqlite3_vec_init)` call before the pool was opened) or the `vec0` \
         module is not compiled in: {0}"
    )]
    NotAvailable(#[source] sqlx::Error),

    /// The probe returned something that is not a version string.
    #[error(
        "sqlite-vec probe returned an unexpected value {0:?}; expected a \
         version string starting with 'v'"
    )]
    UnexpectedVersion(String),
}

/// Error returned when the declared dimension of `vec_memory` cannot be read
/// or does not line up with the configured embedding dimension.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingDimensionError {
    /// `sqlite_master` could not be queried.
    #[error("could not read the vec_memory schema to check its embedding dimension: {0}")]
    SchemaRead(#[source] sqlx::Error),

    /// `vec_memory` is not present in `sqlite_master` (migration not applied?).
    #[error("table vec_memory was not found; run the migrations before checking its dimension")]
    Missing,

    /// The stored DDL does not contain a parseable `float[N]` declaration.
    #[error("could not parse the embedding dimension from the vec_memory DDL: {0:?}")]
    Unparseable(String),

    /// The declared dimension and `EMBEDDING_DIMENSION` disagree. `vec0` has no
    /// `ALTER` for the vector dimension, so the index must be rebuilt.
    #[error(
        "vec_memory declares `embedding float[{declared}]` but EMBEDDING_DIMENSION={expected}: \
         the vector index is out of date and must be rebuilt (drop vec_memory, recreate it \
         with the new dimension, and re-index every memory card)"
    )]
    Mismatch { declared: usize, expected: usize },
}

/// Parse the `float[N]` dimension out of a `vec0` table DDL string.
///
/// `PRAGMA table_info(vec_memory)` reports an **empty** type for the vector
/// column, so the declared dimension is only available in `sqlite_master.sql`.
pub fn declared_dimension_from_sql(sql: &str) -> Option<usize> {
    let start = sql.find("float[")? + "float[".len();
    let rest = &sql[start..];
    let end = rest.find(']')?;
    rest[..end].trim().parse::<usize>().ok()
}

/// Read the dimension declared by the `vec_memory` virtual table.
pub async fn declared_embedding_dimension(
    pool: &SqlitePool,
) -> Result<usize, EmbeddingDimensionError> {
    let sql: Option<String> = sqlx::query_scalar(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='vec_memory'",
    )
    .fetch_optional(pool)
    .await
    .map_err(EmbeddingDimensionError::SchemaRead)?;

    let sql = sql.ok_or(EmbeddingDimensionError::Missing)?;
    declared_dimension_from_sql(&sql).ok_or(EmbeddingDimensionError::Unparseable(sql))
}

/// Compare the declared dimension with the configured one.
///
/// `expected` is `EMBEDDING_DIMENSION` when it is set and parseable; a `None`
/// means the operator has not pinned a dimension, so there is nothing to
/// contradict and the declared one is accepted as-is.
pub fn check_embedding_dimension(
    declared: usize,
    expected: Option<usize>,
) -> Result<(), EmbeddingDimensionError> {
    match expected {
        Some(expected) if expected != declared => {
            Err(EmbeddingDimensionError::Mismatch { declared, expected })
        }
        _ => Ok(()),
    }
}

/// Fail-fast start-up check (D10): verify that the dimension declared by
/// `vec_memory` matches `EMBEDDING_DIMENSION`, so a future embedding-model
/// change cannot silently leave the table misaligned.
pub async fn verify_embedding_dimension(
    pool: &SqlitePool,
) -> Result<usize, EmbeddingDimensionError> {
    let declared = declared_embedding_dimension(pool).await?;
    let expected = std::env::var("EMBEDDING_DIMENSION")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok());
    check_embedding_dimension(declared, expected)?;
    Ok(declared)
}

/// Run the start-up probe against an injectable source.
///
/// This is the testable core of [`verify_vec0`]: the caller supplies the future
/// that runs `SELECT vec_version()`. Production code hands it a pooled query;
/// tests can hand it a failing future to exercise the error branch, which cannot
/// otherwise be reached in-process (see the tests at the bottom of this file).
pub async fn verify_vec0_with<F, Fut>(probe: F) -> Result<String, VecExtensionError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String, sqlx::Error>>,
{
    let version = probe().await.map_err(VecExtensionError::NotAvailable)?;
    if !version.starts_with('v') {
        return Err(VecExtensionError::UnexpectedVersion(version));
    }
    Ok(version)
}

/// Fail-fast start-up check (D11): run `SELECT vec_version()` on the pool and,
/// if it fails, return an error that tells the operator exactly what is missing.
///
/// A `valet` that starts apparently healthy but with no episodic memory is a
/// silent failure this project does not tolerate, so the caller must abort
/// start-up on `Err`.
pub async fn verify_vec0(pool: &SqlitePool) -> Result<String, VecExtensionError> {
    let pool = pool.clone();
    verify_vec0_with(move || async move {
        sqlx::query_scalar::<_, String>("SELECT vec_version()")
            .fetch_one(&pool)
            .await
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    /// Open a fresh in-memory pool. Registration must have happened before this
    /// call for the extension to be present on the opened connections.
    async fn fresh_pool() -> SqlitePool {
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("failed to open in-memory pool")
    }

    /// 2.2 — `SELECT vec_version()` must succeed from a **pooled** connection.
    #[tokio::test]
    async fn vec_version_available_from_pool_connection() {
        register_vec_extension();
        let pool = fresh_pool().await;

        let version = sqlx::query_scalar::<_, String>("SELECT vec_version()")
            .fetch_one(&pool)
            .await
            .expect("SELECT vec_version() should succeed on a pooled connection");

        assert!(
            version.starts_with('v'),
            "expected a version string starting with 'v', got {version:?}"
        );
    }

    /// 2.3 — the registration must reach a **brand-new, standalone connection**
    /// (not a pooled one) opened after registration, because auto-extensions are
    /// applied per connection as it opens.
    #[tokio::test]
    async fn vec_version_available_from_fresh_connection() {
        use sqlx::Connection;

        register_vec_extension();

        // A single connection, opened directly and never handed to a pool:
        // `sqlite3_auto_extension` must still have wired the extension in at
        // open time.
        let mut conn = sqlx::SqliteConnection::connect("sqlite::memory:")
            .await
            .expect("failed to open a standalone connection");

        let version = sqlx::query_scalar::<_, String>("SELECT vec_version()")
            .fetch_one(&mut conn)
            .await
            .expect("SELECT vec_version() should succeed on a fresh connection");

        assert!(
            version.starts_with('v'),
            "expected a version string starting with 'v', got {version:?}"
        );
    }

    /// 2.4 — fail-fast success path: the injectable probe reports a version.
    #[tokio::test]
    async fn verify_vec0_succeeds_when_probe_returns_version() {
        let result = verify_vec0_with(|| async { Ok("v0.1.9".to_string()) }).await;
        assert_eq!(result.expect("probe should succeed"), "v0.1.9");
    }

    /// 2.4 — fail-fast error path: a probe that fails to run the query surfaces
    /// as `NotAvailable`, whose message names the missing extension.
    ///
    /// This exercises the *real* verification/mapping path with a genuinely
    /// failing query result. It is the closest achievable test of the
    /// "extension absent" branch: once `sqlite3_auto_extension` has run it is
    /// process-global and cannot be undone, so a bare connection *without* the
    /// extension cannot be produced in the same process (see module docs).
    #[tokio::test]
    async fn verify_vec0_fails_when_probe_errors() {
        let result = verify_vec0_with(|| async { Err(sqlx::Error::RowNotFound) }).await;

        match result {
            Err(VecExtensionError::NotAvailable(_)) => {}
            other => panic!("expected NotAvailable, got {other:?}"),
        }

        let message = verify_vec0_with(|| async { Err(sqlx::Error::RowNotFound) })
            .await
            .unwrap_err()
            .to_string();
        assert!(
            message.contains("sqlite-vec") && message.contains("vec_version"),
            "error message must name the missing extension, got: {message}"
        );
    }

    /// 2.4 — a probe that returns a non-version string is rejected.
    #[tokio::test]
    async fn verify_vec0_rejects_non_version_string() {
        let result = verify_vec0_with(|| async { Ok("not-a-version".to_string()) }).await;
        assert!(matches!(
            result,
            Err(VecExtensionError::UnexpectedVersion(_))
        ));
    }

    // ─── 3.2 — declared embedding dimension check (D10) ─────────────────────

    /// The `float[N]` dimension is parsed out of the DDL (PRAGMA does not
    /// expose it for `vec0`).
    #[test]
    fn declared_dimension_is_parsed_from_sql() {
        let sql = "CREATE VIRTUAL TABLE vec_memory USING vec0(\n    \
                   id TEXT PRIMARY KEY,\n    \
                   embedding float[1024] distance_metric=cosine\n)";
        assert_eq!(declared_dimension_from_sql(sql), Some(1024));

        let other = "CREATE VIRTUAL TABLE v USING vec0(embedding float[1536])";
        assert_eq!(declared_dimension_from_sql(other), Some(1536));

        assert_eq!(declared_dimension_from_sql("CREATE TABLE t(x TEXT)"), None);
    }

    /// A declared dimension that matches the configured one is accepted.
    #[test]
    fn check_embedding_dimension_aligned_is_ok() {
        assert!(check_embedding_dimension(1024, Some(1024)).is_ok());
        // Nothing configured → nothing to contradict.
        assert!(check_embedding_dimension(1024, None).is_ok());
    }

    /// A declared dimension that disagrees with the configured one fails with a
    /// message that tells the operator to rebuild the index.
    #[test]
    fn check_embedding_dimension_mismatch_fails() {
        let err = check_embedding_dimension(1024, Some(1536)).unwrap_err();
        assert!(matches!(
            err,
            EmbeddingDimensionError::Mismatch {
                declared: 1024,
                expected: 1536
            }
        ));
        let message = err.to_string();
        assert!(
            message.contains("rebuilt"),
            "mismatch message should tell the operator to rebuild, got: {message}"
        );
    }

    /// End to end against a migrated database: the declared dimension is 1024.
    #[tokio::test]
    async fn declared_dimension_of_migrated_db_is_1024() {
        register_vec_extension();
        let pool = fresh_pool().await;
        crate::db::schema::run_migrations(&pool).await.unwrap();

        let declared = declared_embedding_dimension(&pool)
            .await
            .expect("declared dimension should be readable");
        assert_eq!(declared, 1024);
    }
}
