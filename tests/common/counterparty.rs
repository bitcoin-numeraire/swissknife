use std::{
    path::PathBuf,
    process::{Command, Output},
    thread::sleep,
    time::{Duration, Instant},
};

use bitcoin::hashes::{sha256, Hash};
use lightning_invoice::Bolt11Invoice;
use serde_json::{json, Value};
use uuid::Uuid;

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

    pub fn hold_invoice(self, amount_msat: u64) -> HeldInvoice {
        assert!(matches!(self.kind, Kind::Lnd));
        let preimage = sha256::Hash::hash(Uuid::new_v4().as_bytes()).to_byte_array();
        let hash = sha256::Hash::hash(&preimage).to_string();
        let invoice = self.run_json(&[
            "lncli",
            "--network=regtest",
            "addholdinvoice",
            &format!("--amt_msat={amount_msat}"),
            &hash,
        ]);
        HeldInvoice {
            bolt11: invoice["payment_request"].as_str().unwrap().to_string(),
            hash,
            preimage: hex::encode(preimage),
            node: self,
            resolved: false,
        }
    }

    fn cln_rpc(&self, method: &str, params: Value) -> Value {
        assert!(matches!(self.kind, Kind::Cln));
        let params: Vec<String> = params
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| {
                format!(
                    "{key}={}",
                    value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string())
                )
            })
            .collect();
        let mut args = vec!["lightning-cli", "--network=regtest", "-k", method];
        args.extend(params.iter().map(String::as_str));
        self.run_json(&args)
    }

    /// Send a malformed onion in the held payment's group. The real receiving
    /// node rejects this part while the original xpay remains in flight.
    pub fn fail_payment_part(&self, hash: &str) {
        let payments = self.cln_rpc("listsendpays", json!({"payment_hash": hash}));
        let part = payments["payments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|part| part["status"] == "pending")
            .expect("original payment is held");
        let onion = self.cln_rpc(
            "createonion",
            json!({
                "hops": [{"pubkey": part["destination"], "payload": "00"}], "assocdata": hash,
            }),
        );
        self.cln_rpc(
            "sendonion",
            json!({
                "onion": onion["onion"],
                "first_hop": {"id": part["destination"], "amount_msat": part["amount_sent_msat"], "delay": 100},
                "payment_hash": hash, "partid": 999, "groupid": part["groupid"],
            }),
        );
        let started = Instant::now();
        loop {
            let payments = self.cln_rpc("listsendpays", json!({"payment_hash": hash}));
            if payments["payments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|part| part["partid"] == 999 && part["status"] == "failed")
            {
                break;
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "injected part did not fail"
            );
            sleep(Duration::from_millis(100));
        }
        let payment = self.cln_rpc("listpays", json!({"payment_hash": hash}));
        assert_eq!(
            payment["pays"][0]["status"], "pending",
            "the payment itself has not failed"
        );
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

    /// Create an invoice whose payment preimage is known to the test. LNURL
    /// AES success actions use that preimage as their decryption key.
    pub fn invoice_with_preimage(&self, amount_msat: u64, label: &str, preimage: &[u8; 32]) -> String {
        let preimage = hex::encode(preimage);
        match self.kind {
            Kind::Lnd => {
                let amount = format!("--amt_msat={amount_msat}");
                let preimage = format!("--preimage={preimage}");
                let v = self.run_json(&["lncli", "--network=regtest", "addinvoice", &amount, &preimage]);
                v["payment_request"].as_str().expect("lnd payment_request").to_string()
            }
            Kind::Cln => {
                let amount = format!("amount_msat={amount_msat}");
                let label = format!("label={label}");
                let preimage = format!("preimage={preimage}");
                let v = self.run_json(&[
                    "lightning-cli",
                    "--network=regtest",
                    "-k",
                    "invoice",
                    &amount,
                    &label,
                    "description=itest",
                    &preimage,
                ]);
                v["bolt11"].as_str().expect("cln bolt11").to_string()
            }
        }
    }

    /// Ensure the real LND record is settled while SwissKnife is stopped. A
    /// zero-duration client timeout may still have submitted the original RPC;
    /// wait for that attempt before deciding whether the test needs to send it.
    pub fn complete_uncertain_lnd_send(&self, bolt11: &str) {
        assert!(matches!(self.kind, Kind::Lnd));
        let hash = bolt11.parse::<Bolt11Invoice>().unwrap().payment_hash().to_string();
        let started = Instant::now();
        loop {
            let payments = self.run_json(&["lncli", "--network=regtest", "listpayments", "--include_incomplete"]);
            let status = payments["payments"]
                .as_array()
                .unwrap()
                .iter()
                .find(|payment| payment["payment_hash"] == hash)
                .and_then(|payment| payment["status"].as_str());
            match status {
                Some("SUCCEEDED") => return,
                Some("IN_FLIGHT" | "INITIATED") => {
                    assert!(
                        started.elapsed() < Duration::from_secs(30),
                        "LND attempt did not finish"
                    );
                    sleep(Duration::from_millis(100));
                }
                None | Some("FAILED") => {
                    self.pay(bolt11);
                    return;
                }
                other => panic!("Unexpected LND payment status {other:?}"),
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

/// Cancel an unresolved hold even if a test panics, so it does not strand
/// channel liquidity for the remaining integration tests.
pub struct HeldInvoice {
    pub bolt11: String,
    pub hash: String,
    preimage: String,
    node: Counterparty,
    resolved: bool,
}

impl HeldInvoice {
    pub fn accepted(&self) -> bool {
        self.node
            .run_json(&["lncli", "--network=regtest", "lookupinvoice", &self.hash])["state"]
            == "ACCEPTED"
    }

    pub fn settle(&mut self) {
        self.node
            .run(&["lncli", "--network=regtest", "settleinvoice", &self.preimage]);
        self.resolved = true;
    }

    pub fn cancel(&mut self) {
        self.node
            .run(&["lncli", "--network=regtest", "cancelinvoice", &self.hash]);
        self.resolved = true;
    }
}

impl Drop for HeldInvoice {
    fn drop(&mut self) {
        if !self.resolved {
            self.cancel();
        }
    }
}
