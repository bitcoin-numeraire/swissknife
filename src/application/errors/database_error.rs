#[derive(Debug)]
pub enum DatabaseError {
    Connect(String),
}
