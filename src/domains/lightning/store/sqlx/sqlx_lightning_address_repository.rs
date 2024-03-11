use async_trait::async_trait;

use crate::{
    adapters::database::DatabaseClient,
    application::errors::DatabaseError,
    domains::lightning::{entities::LightningAddress, store::LightningAddressRepository},
};

#[derive(Clone)]
pub struct SqlxLightningAddressRepository<D: DatabaseClient> {
    db_client: D,
}

impl<D: DatabaseClient> SqlxLightningAddressRepository<D> {
    pub fn new(db_client: D) -> Self {
        Self { db_client }
    }
}

#[async_trait]
impl<D: DatabaseClient> LightningAddressRepository for SqlxLightningAddressRepository<D> {
    async fn get_by_user_id(
        &self,
        user: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let result
             = sqlx::query_as!(
                LightningAddress,
                r#"
                    SELECT * FROM "lightning_addresses" WHERE user_id = $1
                "#,
                user
            )
            .fetch_optional(&self.db_client.pool()) // fetch_optional for zero or one result
            .await
            .map_err(|e| DatabaseError::Get(e.to_string()))?;
             
        Ok(result)
    }

    async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<LightningAddress>, DatabaseError> {
        let result = sqlx::query_as!(
                LightningAddress,
                r#"
                    SELECT * FROM "lightning_addresses" WHERE username = $1
                "#,
                username,
            )
            .fetch_optional(&self.db_client.pool()) // fetch_optional for zero or one result
            .await
            .map_err(|e| DatabaseError::Get(e.to_string()))?;

        Ok(result)
    }

    async fn list(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let result = sqlx::query_as!(
                LightningAddress,
                r#"
                    SELECT * FROM "lightning_addresses" ORDER BY username LIMIT $1 OFFSET $2
                "#,
                limit as i64, 
                offset as i64   
            )
            .fetch_all(&self.db_client.pool()) // fetch_all for multiple results
            .await
            .map_err(|e| DatabaseError::List(e.to_string()))?;
        
        Ok(result)
    }

    async fn list_by_user_id(
        &self,
        user: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<LightningAddress>, DatabaseError> {
        let result = sqlx::query_as!(
                LightningAddress,
                r#"
                    SELECT * FROM "lightning_addresses" WHERE user_id = $1 ORDER BY username LIMIT $2 OFFSET $3
                "#,
                user,
                limit as i64, 
                offset as i64   
            )
            .fetch_all(&self.db_client.pool()) // fetch_all for multiple results
            .await
            .map_err(|e| DatabaseError::List(e.to_string()))?;

        Ok(result)
    }

    async fn insert(&self, user: &str, username: &str) -> Result<LightningAddress, DatabaseError> {
        let lightning_address = sqlx::query_as!(
            LightningAddress,
            // language=PostgreSQL
            r#"
                insert into "lightning_addresses"(user_id, username)
                values ($1, $2)
                returning *
            "#,
            user,
            username
        )
        .fetch_one(&self.db_client.pool())
        .await
        .map_err(|e| DatabaseError::Insert(e.to_string()))?;

        Ok(lightning_address)
    }
}
