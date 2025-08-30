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

    /// Delete a venue and all its related data by venue slug
    pub async fn delete_venue_data(&self, venue_slug: &str) -> Result<()> {
        let conn = self.get_connection().await?;
        
        // First, find the venue by slug in the data JSON
        let mut rows = conn.query(
            "SELECT id, data FROM nodes WHERE label = 'venue'",
            libsql::params![]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to query venues: {e}")
        })?;
        
        let mut venue_id: Option<String> = None;
        while let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
            message: format!("Failed to read row: {e}"),
        })? {
            let id: String = row.get(0).map_err(|e| ScraperError::Database {
                message: format!("Failed to get id: {e}"),
            })?;
            let data: String = row.get(1).map_err(|e| ScraperError::Database {
                message: format!("Failed to get data: {e}"),
            })?;
            
            // Parse the JSON data to check the slug
            if let Ok(venue_data) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(slug) = venue_data.get("slug").and_then(|s| s.as_str()) {
                    if slug == venue_slug {
                        venue_id = Some(id);
                        break;
                    }
                }
            }
        }
        
        let venue_id = venue_id.ok_or_else(|| ScraperError::Database {
            message: format!("Venue with slug '{}' not found", venue_slug),
        })?;
        
        info!("Found venue '{}' with ID: {}", venue_slug, venue_id);
        
        // Find all events connected to this venue (where venue is source of 'hosts' edge)
        let mut event_ids = Vec::new();
        let mut rows = conn.query(
            "SELECT target_id FROM edges WHERE source_id = ? AND relation = 'hosts'",
            libsql::params![venue_id.clone()]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to query venue events: {e}")
        })?;
        
        while let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
            message: format!("Failed to read row: {e}"),
        })? {
            let event_id: String = row.get(0).map_err(|e| ScraperError::Database {
                message: format!("Failed to get event_id: {e}"),
            })?;
            event_ids.push(event_id);
        }
        
        info!("Found {} events for venue '{}'", event_ids.len(), venue_slug);
        
        // Find all artists connected to these events
        let mut artist_ids = std::collections::HashSet::new();
        for event_id in &event_ids {
            let mut rows = conn.query(
                "SELECT source_id FROM edges WHERE target_id = ? AND relation = 'performs_at'",
                libsql::params![event_id.clone()]
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to query event artists: {e}")
            })?;
            
            while let Some(row) = rows.next().await.map_err(|e| ScraperError::Database {
                message: format!("Failed to read row: {e}"),
            })? {
                let artist_id: String = row.get(0).map_err(|e| ScraperError::Database {
                    message: format!("Failed to get artist_id: {e}"),
                })?;
                artist_ids.insert(artist_id);
            }
        }
        
        info!("Found {} unique artists for venue '{}'", artist_ids.len(), venue_slug);
        
        // Now delete everything in order:
        // 1. Delete all edges related to the venue, events, and artists
        conn.execute(
            "DELETE FROM edges WHERE source_id = ? OR target_id = ?",
            libsql::params![venue_id.clone(), venue_id.clone()]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to delete venue edges: {e}")
        })?;
        
        // 2. Delete edges for all events
        for event_id in &event_ids {
            conn.execute(
                "DELETE FROM edges WHERE source_id = ? OR target_id = ?",
                libsql::params![event_id.clone(), event_id.clone()]
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to delete event edges: {e}")
            })?;
        }
        
        // 3. Delete edges for all artists (only if they're not connected to other venues)
        for artist_id in &artist_ids {
            // Check if this artist performs at events from other venues
            let mut rows = conn.query(
                "SELECT e.target_id FROM edges e 
                 JOIN edges v ON e.target_id = v.target_id 
                 WHERE e.source_id = ? AND e.relation = 'performs_at' 
                 AND v.relation = 'hosts' AND v.source_id != ?",
                libsql::params![artist_id.clone(), venue_id.clone()]
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to check artist connections: {e}")
            })?;
            
            let has_other_venues = rows.next().await.map_err(|e| ScraperError::Database {
                message: format!("Failed to read row: {e}"),
            })?.is_some();
            
            if !has_other_venues {
                // Delete artist edges if not connected to other venues
                conn.execute(
                    "DELETE FROM edges WHERE source_id = ? OR target_id = ?",
                    libsql::params![artist_id.clone(), artist_id.clone()]
                )
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to delete artist edges: {e}")
                })?;
            }
        }
        
        // 4. Delete event nodes
        for event_id in &event_ids {
            conn.execute(
                "DELETE FROM nodes WHERE id = ?",
                libsql::params![event_id.clone()]
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to delete event node: {e}")
            })?;
        }
        
        // 5. Delete artist nodes (only if not connected to other venues)
        for artist_id in &artist_ids {
            // Check again if artist has other connections
            let mut rows = conn.query(
                "SELECT id FROM edges WHERE (source_id = ? OR target_id = ?) LIMIT 1",
                libsql::params![artist_id.clone(), artist_id.clone()]
            )
            .await
            .map_err(|e| ScraperError::Database {
                message: format!("Failed to check artist edges: {e}")
            })?;
            
            let has_edges = rows.next().await.map_err(|e| ScraperError::Database {
                message: format!("Failed to read row: {e}"),
            })?.is_some();
            
            if !has_edges {
                conn.execute(
                    "DELETE FROM nodes WHERE id = ?",
                    libsql::params![artist_id.clone()]
                )
                .await
                .map_err(|e| ScraperError::Database {
                    message: format!("Failed to delete artist node: {e}")
                })?;
            }
        }
        
        // 6. Delete the venue node
        conn.execute(
            "DELETE FROM nodes WHERE id = ?",
            libsql::params![venue_id.clone()]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to delete venue node: {e}")
        })?;
        
        info!("Successfully deleted venue '{}' and related data", venue_slug);
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

    /// Delete a node by ID
    pub async fn delete_node(&self, node_id: &str) -> Result<()> {
        let conn = self.get_connection().await?;
        
        // First delete all edges related to this node
        conn.execute(
            "DELETE FROM edges WHERE source_id = ? OR target_id = ?",
            libsql::params![node_id, node_id]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to delete edges for node {}: {}", node_id, e),
        })?;
        
        // Then delete the node itself
        conn.execute(
            "DELETE FROM nodes WHERE id = ?",
            libsql::params![node_id]
        )
        .await
        .map_err(|e| ScraperError::Database {
            message: format!("Failed to delete node {}: {}", node_id, e),
        })?;
        
        Ok(())
    }
}
