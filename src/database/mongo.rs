//! MongoDB database wrapper.

use mongodb::{options::ClientOptions, Client, Collection};
use tracing::info;

/// Database wrapper for MongoDB operations.
#[derive(Debug, Clone)]
pub struct Database {
    client: Client,
    db: mongodb::Database,
}

impl Database {
    /// Connect to MongoDB with the given URI and database name.
    ///
    /// # Arguments
    /// * `uri` - MongoDB connection string
    /// * `db_name` - Database name to use
    ///
    /// # Errors
    /// Returns error if connection fails.
    pub async fn connect(uri: &str, db_name: &str) -> anyhow::Result<Self> {
        let options = ClientOptions::parse(uri).await?;
        let client = Client::with_options(options)?;

        // Ping the database to verify connection
        client
            .database("admin")
            .run_command(mongodb::bson::doc! { "ping": 1 })
            .await?;

        info!("Successfully connected to MongoDB");

        let db = client.database(db_name);

        Ok(Self { client, db })
    }

    /// Get a reference to the underlying MongoDB client.
    #[allow(dead_code)]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Get a reference to the database.
    #[allow(dead_code)]
    pub fn db(&self) -> &mongodb::Database {
        &self.db
    }

    /// Get a typed collection from the database.
    ///
    /// # Arguments
    /// * `name` - Collection name
    #[allow(dead_code)]
    pub fn collection<T: Send + Sync>(&self, name: &str) -> Collection<T> {
        self.db.collection(name)
    }
}
