use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

/// The Lightning node NOT under test, driven via its CLI through docker so that
/// real payments flow over the LND<->CLN channel. When SwissKnife runs on LND
/// the counterparty is CLN, and vice-versa, so every provider has a live peer.
pub struct Counterparty {
    project: String,
    compose_file: String,
    kind: Kind,
}

enum Kind {
    Lnd,
    Cln,
}

impl Counterparty {
    pub fn for_provider(provider: &str) -> Self {
        let kind = if provider.starts_with("lnd") {
            Kind::Cln
        } else {
            Kind::Lnd
        };
        let project =
            std::env::var("SWISSKNIFE_ITEST_COMPOSE_PROJECT").unwrap_or_else(|_| "swissknife-itest".to_string());
        let compose_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/itest/docker-compose.yml")
            .display()
            .to_string();
        Self {
            project,
            compose_file,
            kind,
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        let service = match self.kind {
            Kind::Lnd => "lnd",
            Kind::Cln => "cln",
        };
        let mut cmd = Command::new("docker");
        cmd.arg("compose")
            .arg("-p")
            .arg(&self.project)
            .arg("-f")
            .arg(&self.compose_file)
            .arg("exec")
            .arg("-T")
            .arg(service);
        for arg in args {
            cmd.arg(arg);
        }
        let output = cmd.output().expect("run counterparty CLI via docker compose exec");
        assert!(
            output.status.success(),
            "counterparty command {args:?} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn run_json(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
            panic!(
                "counterparty command {args:?} returned non-JSON ({err}): {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    /// Create an invoice on the counterparty and return its bolt11 (for the SUT to pay).
    pub fn invoice(&self, amount_msat: u64, label: &str) -> String {
        match self.kind {
            Kind::Lnd => {
                let v = self.run_json(&[
                    "lncli",
                    "--network=regtest",
                    "addinvoice",
                    &format!("--amt_msat={amount_msat}"),
                ]);
                v["payment_request"].as_str().expect("lnd payment_request").to_string()
            }
            Kind::Cln => {
                let v = self.run_json(&[
                    "lightning-cli",
                    "--network=regtest",
                    "invoice",
                    &amount_msat.to_string(),
                    label,
                    "itest",
                ]);
                v["bolt11"].as_str().expect("cln bolt11").to_string()
            }
        }
    }

    /// Pay a bolt11 from the counterparty (so the SUT receives).
    pub fn pay(&self, bolt11: &str) {
        match self.kind {
            Kind::Lnd => {
                self.run(&["lncli", "--network=regtest", "payinvoice", "--force", bolt11]);
            }
            Kind::Cln => {
                self.run(&["lightning-cli", "--network=regtest", "pay", bolt11]);
            }
        }
    }
}
