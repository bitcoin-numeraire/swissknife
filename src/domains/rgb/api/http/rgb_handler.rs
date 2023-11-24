use axum::{extract::State, routing::post, Json, Router};

use serde::{Deserialize, Serialize};

use crate::{
    adapters::rgb::DynRGBClient, application::errors::ApplicationError,
    domains::rgb::entities::RGBContract,
};

pub struct RGBHandler;

impl RGBHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn routes(&self, rgb_client: DynRGBClient) -> Router {
        Router::new()
            .route("/contracts/issue", post(issue_contract))
            .with_state(rgb_client)
    }
}

async fn issue_contract(
    State(rgb_client): State<DynRGBClient>,
    Json(payload): Json<IssueContractRequest>,
) -> Result<Json<ContractResponse>, ApplicationError> {
    println!("Issuing contract: {:?}", payload);

    let contract_id = match rgb_client
        .issue_contract(
            payload.url,
            RGBContract {
                ticker: "TST".to_string(),
                name: "Test".to_string(),
                precision: 0,
                amounts: vec![100000],
            },
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error issuing contract: {:?}", e);
            return Err(e);
        }
    };

    println!("Contract issued: {}", contract_id);

    let contract = ContractResponse {
        id: contract_id,
        username: payload.username,
    };

    Ok(contract.into())
}

#[derive(Debug, Deserialize)]
struct IssueContractRequest {
    username: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct ContractResponse {
    id: String,
    username: String,
}
