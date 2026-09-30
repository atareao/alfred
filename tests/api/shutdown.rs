//! Regression test for orderly shutdown on `SIGTERM`.
//!
//! `valet` normally runs as PID 1 inside its container. The kernel does **not**
//! apply the default signal action to PID 1, so a bare `SIGTERM` sent by
//! `podman stop` / `docker stop` would be ignored and the runtime would
//! escalate to `SIGKILL` once its 10-second grace period expires. The server
//! therefore installs its own handlers and runs an orderly shutdown path.
//!
//! **What this test asserts and why:** a normal child process is *not* PID 1,
//! so `SIGTERM` kills it through the kernel's default action even when no
//! handler is installed. Asserting merely that "the process died after
//! `SIGTERM`" would therefore pass even against a broken shutdown path and
//! would protect nothing. Instead this test asserts on the log line that is
//! emitted only when the orderly shutdown actually runs. That line is what
//! distinguishes the container (PID 1) behaviour we care about: with the
//! default action the process dies silently and the marker never appears.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Log line written once the server is bound and about to serve. It is emitted
/// after the signal handlers have been installed, so seeing it removes the race
/// between spawning the process and sending `SIGTERM`.
const LISTEN_MARKER: &str = "Valet server listening on";

/// Log line written only when the orderly shutdown path runs to completion.
const CLEAN_SHUTDOWN_MARKER: &str = "Valet server shut down cleanly";

/// How long to let the server run with **no** signal before checking that it is
/// still alive. This must comfortably exceed the production
/// `SHUTDOWN_DRAIN_TIMEOUT` (5s): the bug this guards against was a drain
/// timeout placed around the whole `serve` future, which fired at 5s and killed
/// a healthy server without any signal.
const SURVIVAL_WAIT: Duration = Duration::from_secs(8);

/// Build a unique temporary path for this test run so repeated runs cannot
/// collide with each other or with leftovers from a previous failure.
fn temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be after the Unix epoch")
        .as_nanos();
    let mut path = std::env::temp_dir();
    path.push(format!(
        "valet_shutdown_{label}_{}_{nanos}.tmp",
        std::process::id()
    ));
    path
}

fn read_log(path: &PathBuf) -> String {
    // A missing file simply means "nothing logged yet".
    fs::read_to_string(path).unwrap_or_default()
}

/// Parse the ephemeral port out of the
/// `Valet server listening on 127.0.0.1:<port>` line. Returns `None` until
/// that line is present. The server is spawned with `PORT=0`, so the real port
/// is only discoverable from this log line.
fn parse_listening_port(log: &str) -> Option<u16> {
    let marker = format!("{LISTEN_MARKER} ");
    let start = log.find(&marker)? + marker.len();
    let rest = &log[start..];
    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    rest[..end].rsplit(':').next()?.parse().ok()
}

/// Minimal hand-written HTTP/1.1 `GET /api/health`, returning `true` only when
/// the server answers with `200`. Deliberately dependency-free: adding an HTTP
/// client just for this regression check is not worth it.
fn health_returns_200(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(stream) => stream,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let request =
        format!("GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if stream.write_all(request.as_bytes()).is_err() {
        return false;
    }

    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }

    response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200")
}

/// Poll the log file until it contains `marker`, up to `timeout`.
fn wait_for_log_marker(log_path: &PathBuf, marker: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if read_log(log_path).contains(marker) {
            return true;
        }
        sleep(Duration::from_millis(100));
    }
    false
}

/// Owns the spawned child and the temporary files, so they are always cleaned
/// up even if an assertion panics midway through the test.
struct Cleanup {
    child: Child,
    files: Vec<PathBuf>,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        // Kill the child if it is still alive, then reap it to avoid a zombie.
        let _ = self.child.kill();
        let _ = self.child.wait();
        for path in &self.files {
            let _ = fs::remove_file(path);
        }
    }
}

#[test]
fn test_sigterm_runs_orderly_shutdown() {
    // -----------------------------------------------------------------
    // Arrange: a unique database and log file per run, with leftovers
    //          from a previous (possibly failed) run removed first.
    // -----------------------------------------------------------------
    let log_path = temp_path("log");
    let db_path = temp_path("db");
    let _ = fs::remove_file(&log_path);
    for suffix in ["", "-wal", "-shm"] {
        let _ = fs::remove_file(format!("{}{suffix}", db_path.display()));
    }

    let log_file = File::create(&log_path).expect("create temporary log file");
    let log_file_err = log_file.try_clone().expect("clone log file handle");

    let binary = env!("CARGO_BIN_EXE_valet");
    let child = Command::new(binary)
        .env("DATABASE_URL", db_path.to_str().expect("db path is UTF-8"))
        .env("HOST", "127.0.0.1")
        // Port 0 lets the OS pick a free port; the real port is read back
        // from the `listening` log line so the survival check can talk to it.
        .env("PORT", "0")
        .env("AUTH_ENABLED", "false")
        .env("OPENROUTER_API_KEY", "sk-test-shutdown")
        .env("RUST_LOG", "info")
        // Redirect BOTH streams to the file. Do NOT use `Stdio::piped()`
        // without draining it: this binary logs enough to fill the pipe buffer
        // and block on write, deadlocking the child.
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .expect("spawn the valet binary");

    let mut cleanup = Cleanup {
        child,
        files: vec![
            log_path.clone(),
            db_path.clone(),
            PathBuf::from(format!("{}-wal", db_path.display())),
            PathBuf::from(format!("{}-shm", db_path.display())),
        ],
    };

    // -----------------------------------------------------------------
    // Wait until the server is really listening. This guarantees the
    // signal handlers are installed before we send anything.
    // -----------------------------------------------------------------
    assert!(
        wait_for_log_marker(&log_path, LISTEN_MARKER, Duration::from_secs(20)),
        "server never logged `{LISTEN_MARKER}` within 20s; log contents:\n{}",
        read_log(&log_path)
    );

    // -----------------------------------------------------------------
    // Survival check (no signal yet): the server must keep serving well
    // past the drain budget. `SHUTDOWN_DRAIN_TIMEOUT` is 5s, so waiting 8s
    // and still getting a 200 proves the drain clock is NOT running during
    // normal service.
    //
    // This step exists precisely because a timeout placed around the whole
    // `serve` future fires at ~5s and kills a healthy server with no signal
    // at all. Without this wait the test only observes a few milliseconds of
    // uptime and cannot detect that regression.
    // -----------------------------------------------------------------
    sleep(SURVIVAL_WAIT);

    match cleanup.child.try_wait() {
        Ok(Some(status)) => panic!(
            "server exited on its own ~{}s after startup with NO signal \
             (status {status}); it did not survive the drain budget. Log contents:\n{}",
            SURVIVAL_WAIT.as_secs(),
            read_log(&log_path)
        ),
        Ok(None) => {}
        Err(error) => panic!("try_wait failed: {error}"),
    }

    let port = parse_listening_port(&read_log(&log_path)).unwrap_or_else(|| {
        panic!(
            "could not parse the listening port from the log after startup:\n{}",
            read_log(&log_path)
        )
    });
    assert!(
        health_returns_200(port),
        "server did not survive the drain budget: `GET /api/health` on \
         127.0.0.1:{port} did not return 200 ~{}s after startup with NO shutdown \
         signal. This means the server shut itself down inside \
         SHUTDOWN_DRAIN_TIMEOUT without a signal (the drain timeout is wrapping \
         normal service). Log contents:\n{}",
        SURVIVAL_WAIT.as_secs(),
        read_log(&log_path)
    );

    // -----------------------------------------------------------------
    // Act: send SIGTERM, just like `podman stop` would.
    // -----------------------------------------------------------------
    let pid = cleanup.child.id();
    let kill_status = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .expect("run `kill`");
    assert!(kill_status.success(), "`kill -TERM {pid}` failed");

    // -----------------------------------------------------------------
    // Wait (bounded) for the process to exit.
    // -----------------------------------------------------------------
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut exited = false;
    while Instant::now() < deadline {
        match cleanup.child.try_wait() {
            Ok(Some(_status)) => {
                exited = true;
                break;
            }
            Ok(None) => sleep(Duration::from_millis(50)),
            Err(error) => panic!("try_wait failed: {error}"),
        }
    }

    // -----------------------------------------------------------------
    // Assert: the orderly shutdown actually ran.
    //
    // This is the real regression check. With the default SIGTERM action the
    // process dies just the same, but it dies *without* executing the shutdown
    // path, so `CLEAN_SHUTDOWN_MARKER` is absent. That is exactly the failure
    // mode that would leave a PID 1 container being SIGKILLed.
    // -----------------------------------------------------------------
    assert!(
        exited,
        "process {pid} did not exit within 5s of SIGTERM; log contents:\n{}",
        read_log(&log_path)
    );

    let log = read_log(&log_path);
    assert!(
        log.contains(CLEAN_SHUTDOWN_MARKER),
        "orderly shutdown did not run: `{CLEAN_SHUTDOWN_MARKER}` missing from the log. \
         The process exited, but via the default signal action rather than the \
         installer's handler. Log contents:\n{log}"
    );
}
