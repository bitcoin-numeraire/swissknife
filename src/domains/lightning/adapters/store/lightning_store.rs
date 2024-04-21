use sea_orm::DatabaseConnection;

#[derive(Clone)]
pub struct LightningStore {
    pub db: DatabaseConnection,
}

impl LightningStore {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }
}
