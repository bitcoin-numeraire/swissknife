use std::{
    env,
    fs::{self, File},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::Duration,
};

use serde_json::json;
use tokio::{
    sync::OnceCell,
    time::{sleep, Instant},
};

use super::client::{ApiClient, Auth};
use super::db::TestDatabase;

/// Password for the bootstrap admin user created during [`TestApp::start`].
pub const ADMIN_PASSWORD: &str = "integration-admin-password";

const STARTUP_TIMEOUT: Duration = Duration::from_secs(90);
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Instances we spawned. Statics are never dropped, so we kill them explicitly
/// at process exit — a shared instance would otherwise leak past the test run.
static SPAWNED: Mutex<Vec<Child>> = Mutex::new(Vec::new());

// `method = at_binary_exit` registers via libc `atexit` (the behavior of the
// old `ctor::dtor`), not dtor's default `.fini_array` linker section. This runs
// the hook while std services are still alive, so the `Mutex`/`Command`/wait
// calls below are safe; a linker destructor could run after teardown and leak
// the spawned child processes.
#[dtor::dtor(unsafe, method = at_binary_exit)]
fn reap_spawned_instances() {
    let Ok(mut children) = SPAWNED.lock() else {
        return;
    };
    // SIGTERM first so SwissKnife shuts down gracefully. A hard SIGKILL would
    // stop the instrumented binary from running its atexit handler, dropping
    // the coverage profile under `cargo llvm-cov`.
    for child in children.iter_mut() {
        let _ = std::process::Command::new("kill").arg(child.id().to_string()).status();
    }
    for mut child in children.drain(..) {
        for _ in 0..50 {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = child.kill(); // hard-kill any straggler that ignored SIGTERM
        let _ = child.wait();
    }
}

static APP: OnceCell<TestApp> = OnceCell::const_new();

/// The process-wide shared SwissKnife instance, spawned on first use. Tests
/// share it and isolate by creating uniquely-named entities.
pub async fn app() -> &'static TestApp {
    APP.get_or_init(TestApp::start).await
}

pub struct TestApp {
    pub base_url: String,
    pub database: String,
    pub database_url: String,
    pub provider: String,
    admin_jwt: String,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

/// The `(database, provider)` matrix cell under test, from env (defaults
/// `sqlite` / `lnd_grpc`). Shared by every spawned instance.
pub fn matrix_cell() -> (String, String) {
    let database = env::var("SWISSKNIFE_ITEST_DATABASE").unwrap_or_else(|_| "sqlite".to_string());
    let provider = env::var("SWISSKNIFE_ITEST_PROVIDER").unwrap_or_else(|_| "lnd_grpc".to_string());
    (database, provider)
}

/// A spawned, ready SwissKnife instance: its base URL and log paths.
pub struct Spawned {
    pub base_url: String,
    pub database_url: String,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

/// Spawn a SwissKnife instance against the regtest stack with `extra_env`
/// layered on top of the per-instance dynamics, returning once it reports ready.
///
/// Only the per-instance dynamics are passed as env; everything else comes from
/// config/itest.toml (RUN_MODE=itest). cwd=repo root so the config's relative
/// cert/macaroon paths resolve. `label` names the per-instance database and log
/// files, so distinct instances in one test process never collide. The child is
/// registered for graceful reaping at process exit.
pub async fn spawn_instance(database: &str, provider: &str, label: &str, extra_env: &[(&str, String)]) -> Spawned {
    let root = repo_root();

    let db = TestDatabase::provision(database, &root, label).await;
    let database_url = db.url().to_string();
    let port = free_port();
    let base_url = format!("http://127.0.0.1:{port}");

    let artifacts = root.join("target/itest");
    fs::create_dir_all(&artifacts).expect("create itest artifact dir");
    let stdout_path = artifacts.join(format!("swissknife-{label}.stdout.log"));
    let stderr_path = artifacts.join(format!("swissknife-{label}.stderr.log"));

    let mut command = Command::new(env!("CARGO_BIN_EXE_swissknife"));
    command
        .current_dir(&root)
        .stdout(Stdio::from(File::create(&stdout_path).expect("create stdout log")))
        .stderr(Stdio::from(File::create(&stderr_path).expect("create stderr log")))
        .env("RUN_MODE", "itest")
        .env("SWISSKNIFE_WEB__ADDR", format!("127.0.0.1:{port}"))
        .env("SWISSKNIFE_DATABASE__URL", db.url())
        .env("SWISSKNIFE_LN_PROVIDER", provider)
        // Advertise this ephemeral server as the public host so the callback
        // URL the LNURL well-known endpoint hands out is actually reachable.
        .env("SWISSKNIFE_HOST", &base_url);

    // The CLN rune is generated at bootstrap and cannot be a file path.
    if provider == "cln_rest" {
        command.env("SWISSKNIFE_CLN_REST_CONFIG__RUNE", read_cln_rune(&root));
    }
    for (key, value) in extra_env {
        command.env(key, value);
    }

    let child = command
        .spawn()
        .expect("spawn swissknife binary (run `make build` first)");
    SPAWNED.lock().expect("spawned registry lock").push(child);

    let api = ApiClient::new(base_url.clone());
    wait_until_ready(&api, &stdout_path, &stderr_path).await;

    Spawned {
        base_url,
        database_url,
        stdout_path,
        stderr_path,
    }
}

impl TestApp {
    async fn start() -> TestApp {
        let (database, provider) = matrix_cell();
        let label = format!("{database}-{provider}");
        let spawned = spawn_instance(&database, &provider, &label, &[]).await;

        // Create the admin once, up front, so no test races on its creation.
        let api = ApiClient::new(spawned.base_url.clone());
        let admin_jwt = bootstrap_admin(&api).await;

        TestApp {
            base_url: spawned.base_url,
            database,
            database_url: spawned.database_url,
            provider,
            admin_jwt,
            stdout_path: spawned.stdout_path,
            stderr_path: spawned.stderr_path,
        }
    }

    /// A fresh HTTP client bound to this instance.
    pub fn api(&self) -> ApiClient {
        ApiClient::new(self.base_url.clone())
    }

    /// JWT for the bootstrap admin (all permissions), created during startup.
    pub async fn admin_token(&self) -> &str {
        &self.admin_jwt
    }
}

/// Sign up the first admin (all permissions); fall back to sign-in if a prior
/// run already created it (e.g. a reused database).
async fn bootstrap_admin(api: &ApiClient) -> String {
    let signup = api
        .post("/v1/auth/sign-up", Auth::None, json!({ "password": ADMIN_PASSWORD }))
        .await;
    let body = match signup.status.as_u16() {
        200 => signup.body,
        409 => {
            api.post("/v1/auth/sign-in", Auth::None, json!({ "password": ADMIN_PASSWORD }))
                .await
                .body
        }
        other => panic!("admin bootstrap: unexpected sign-up status {other}: {}", signup.body),
    };
    let token = body["token"]
        .as_str()
        .expect("auth response contains a token")
        .to_string();

    // The admin's wallet is provisioned lazily on the first authenticated
    // request, and that path is a non-atomic find-then-insert (see the itest
    // follow-up issue). Trigger it once here, serially, so parallel tests never
    // race to create the same wallet.
    let warmup = api.get("/v1/me", Auth::Bearer(&token)).await;
    assert_eq!(
        warmup.status.as_u16(),
        200,
        "admin warmup (/v1/me) failed: {}",
        warmup.body
    );

    token
}

async fn wait_until_ready(api: &ApiClient, stdout_path: &Path, stderr_path: &Path) {
    let started = Instant::now();
    loop {
        if let Ok(res) = api.try_get("/v1/system/ready").await {
            if res.status == reqwest::StatusCode::NO_CONTENT {
                return;
            }
        }
        assert!(
            started.elapsed() <= STARTUP_TIMEOUT,
            "swissknife did not become ready within {STARTUP_TIMEOUT:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
            tail(stdout_path),
            tail(stderr_path),
        );
        sleep(POLL_INTERVAL).await;
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind ephemeral port")
        .local_addr()
        .expect("read local addr")
        .port()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_cln_rune(root: &Path) -> String {
    fs::read_to_string(root.join("tests/itest/runtime/cln/rune"))
        .expect("CLN rune missing — run `make itest-up` first")
        .trim()
        .to_string()
}

fn tail(path: &Path) -> String {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut lines: Vec<&str> = content.lines().rev().take(60).collect();
    lines.reverse();
    lines.join("\n")
}
