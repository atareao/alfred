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
}
