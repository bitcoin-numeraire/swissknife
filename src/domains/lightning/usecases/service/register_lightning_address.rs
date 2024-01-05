use async_trait::async_trait;
use tracing::debug;

use crate::{
    application::errors::LightningError,
    domains::lightning::{entities::LightningAddress, usecases::LightningUseCases},
};

use super::LightningService;

#[async_trait]
impl LightningUseCases for LightningService {
    async fn register_lightning_address(
        &self,
        user_id: String,
        username: String,
    ) -> Result<LightningAddress, LightningError> {
        debug!(user_id, username, "Registering lightning address");

        let lightning_address = sqlx::query_as!(
            LightningAddress,
            // language=PostgreSQL
            r#"
                insert into "lightning_addresses"(user_id, username)
                values ($1, $2)
                returning *
            "#,
            user_id,
            username
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| {
            let err_message = "Database error";
            debug!(error = ?e, err_message);
            LightningError::Register(e.to_string())
        })?;

        Ok(lightning_address)
    }
}
