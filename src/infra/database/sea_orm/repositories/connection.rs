use sea_orm::{ConnectionTrait, DatabaseConnection, DatabaseTransaction};

pub(crate) trait SeaOrmConnection: Send + Sync {
    type Connection: ConnectionTrait;

    fn connection(&self) -> &Self::Connection;
}

impl SeaOrmConnection for DatabaseConnection {
    type Connection = DatabaseConnection;

    fn connection(&self) -> &Self::Connection {
        self
    }
}

impl SeaOrmConnection for &DatabaseTransaction {
    type Connection = DatabaseTransaction;

    fn connection(&self) -> &Self::Connection {
        self
    }
}
