mod conn;
pub(crate) mod driver;

use anyhow::Result;
use diesel::r2d2::{ConnectionManager, PooledConnection};
use diesel::sqlite::SqliteConnection;
#[cfg(test)]
pub(crate) use driver::tests::TestDriver;
pub(crate) use driver::Driver;

use crate::Service;

/// Sqlite provides database access functionality.
/// It abstracts away the connection pooling details and
/// provides a simple interface for getting database connections.
#[async_trait::async_trait]
pub trait Sqlite: Send + Sync {
    /// Gets a connection from the pool.
    async fn connection(&self) -> Result<PooledConnection<ConnectionManager<SqliteConnection>>>;
}

impl Service {
    /// Create a new SQLite service with custom timeout
    pub fn db_pool_service(db_path: &str, timeout: std::time::Duration) -> Result<impl Sqlite> {
        Driver::new(db_path, Some(timeout))
    }
}
