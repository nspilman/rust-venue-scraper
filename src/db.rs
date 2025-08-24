use crate::common::error::{Result, ScraperError};
use libsql::{Builder, Connection, Database};
use std::env;
use tracing::info;

pub struct DatabaseManager {
    db: Database,
}

impl DatabaseManager {
    /// Create a new database manager with connection to Turso
    pub async fn new() -> Result<Self> {
        let url = env::var("LIBSQL_URL").map_err(|_| ScraperError::Database {
            message: "LIBSQL_URL environment variable not set".to_string(),
        })?;

        let auth_token = env::var("LIBSQL_AUTH_TOKEN").map_err(|_| ScraperError::Database {
            message: "LIBSQL_AUTH_TOKEN environment variable not set".to_string(),
        })?;

        info!("Connecting to Turso database at {}", url);

        let db = Builder::new_remote(url, auth_token)
            .build()
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to connect to database: {e}"),
            })?;

        Ok(Self { db })
    }

    /// Get a connection to the database
    pub async fn get_connection(&self) -> Result<Connection> {
        self.db.connect().map_err(|e| ScraperError::Database {
            message: format!("Failed to get database connection: {e}"),
        })
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        let conn = self.get_connection().await?;

        // Apply base schema
        let migration_sql_001 = include_str!("../migrations/001_create_nodes_and_edges.sql");
        conn.execute_batch(migration_sql_001)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to run base migration: {e}"),
            })?;

        // Apply indexes and PRAGMAs
        let migration_sql_002 = include_str!("../migrations/002_indexes_and_pragmas.sql");
        conn.execute_batch(migration_sql_002)
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to run index migration: {e}"),
            })?;

        info!("Database migrations completed successfully");
        Ok(())
    }

    /// Create or update a node in the database (upsert)
    pub async fn create_node(&self, id: &str, label: &str, data: &str) -> Result<()> {
        let conn = self.get_connection().await?;

        // Use explicit ON CONFLICT(id) DO UPDATE to avoid destructive REPLACE semantics
        conn.execute(
            "INSERT INTO nodes (id, label, data, created_at, updated_at)
             VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM nodes WHERE id = ?1), datetime('now')), datetime('now'))
             ON CONFLICT(id) DO UPDATE SET
               data = excluded.data,
               updated_at = excluded.updated_at",
            libsql::params![id, label, data]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to upsert node: {e}")
        })?;

        Ok(())
    }

    /// Create or update an edge in the database (upsert)
    pub async fn create_edge(
        &self,
        id: &str,
        source_id: &str,
        target_id: &str,
        relation: &str,
        data: Option<&str>,
    ) -> Result<()> {
        let conn = self.get_connection().await?;

        // Use unique (source_id, target_id, relation) to idempotently upsert edges
        conn.execute(
            "INSERT INTO edges (id, source_id, target_id, relation, data, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, COALESCE((SELECT created_at FROM edges WHERE source_id = ?2 AND target_id = ?3 AND relation = ?4), datetime('now')), datetime('now'))
             ON CONFLICT(source_id, target_id, relation) DO UPDATE SET
               data = excluded.data,
               updated_at = excluded.updated_at",
            libsql::params![id, source_id, target_id, relation, data]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to upsert edge: {e}")
        })?;

        Ok(())
    }

    /// Get a node by ID
    pub async fn get_node(&self, id: &str) -> Result<Option<(String, String, String)>> {
        let conn = self.get_connection().await?;

        let mut rows = conn
            .query(
                "SELECT id, label, data FROM nodes WHERE id = ?",
                libsql::params![id],
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query node: {e}"),
            })?;

        if let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
            message: format!("Failed to read row: {e}"),
        })? {
            let id: String = row.get(0).map_err(|e| ScraperError::Database {
                message: format!("Failed to get id: {e}"),
            })?;
            let label: String = row.get(1).map_err(|e| ScraperError::Database {
                message: format!("Failed to get label: {e}"),
            })?;
            let data: String = row.get(2).map_err(|e| ScraperError::Database {
                message: format!("Failed to get data: {e}"),
            })?;

            Ok(Some((id, label, data)))
        } else {
            Ok(None)
        }
    }

    /// Get all nodes by label
    pub async fn get_nodes_by_label(&self, label: &str) -> Result<Vec<(String, String, String)>> {
        let conn = self.get_connection().await?;

        let mut rows = conn
            .query(
                "SELECT id, label, data FROM nodes WHERE label = ?",
                libsql::params![label],
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query nodes: {e}"),
            })?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
            message: format!("Failed to read row: {e}"),
        })? {
            let id: String = row.get(0).map_err(|e| ScraperError::Database {
                message: format!("Failed to get id: {e}"),
            })?;
            let label: String = row.get(1).map_err(|e| ScraperError::Database {
                message: format!("Failed to get label: {e}"),
            })?;
            let data: String = row.get(2).map_err(|e| ScraperError::Database {
                message: format!("Failed to get data: {e}"),
            })?;

            results.push((id, label, data));
        }

        Ok(results)
    }

    /// Clear all data from the database (useful for development)
    pub async fn clear_all_data(&self) -> Result<()> {
        let conn = self.get_connection().await?;

        // Delete all edges first (foreign key constraints)
        conn.execute("DELETE FROM edges", libsql::params![])
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to clear edges: {e}"),
            })?;

        // Delete all nodes
        conn.execute("DELETE FROM nodes", libsql::params![])
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to clear nodes: {e}"),
            })?;

        info!("Cleared all data from database");
        Ok(())
    }

    /// Get edges for a node
    pub async fn get_edges_for_node(
        &self,
        node_id: &str,
    ) -> Result<Vec<(String, String, String, String, Option<String>)>> {
        let conn = self.get_connection().await?;

        let mut rows = conn.query(
            "SELECT id, source_id, target_id, relation, data FROM edges WHERE source_id = ? OR target_id = ?",
            libsql::params![node_id, node_id]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to query edges: {e}")
        })?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
            message: format!("Failed to read row: {e}"),
        })? {
            let id: String = row.get(0).map_err(|e| ScraperError::Database {
                message: format!("Failed to get id: {e}"),
            })?;
            let source_id: String = row.get(1).map_err(|e| ScraperError::Database {
                message: format!("Failed to get source_id: {e}"),
            })?;
            let target_id: String = row.get(2).map_err(|e| ScraperError::Database {
                message: format!("Failed to get target_id: {e}"),
            })?;
            let relation: String = row.get(3).map_err(|e| ScraperError::Database {
                message: format!("Failed to get relation: {e}"),
            })?;
            let data: Option<String> = row.get(4).ok();

            results.push((id, source_id, target_id, relation, data));
        }

        Ok(results)
    }
}
