-- Create nodes table
-- This table stores individual entities (venues, events, artists, etc.)
CREATE TABLE IF NOT EXISTS nodes (
    id TEXT PRIMARY KEY,           -- UUID as TEXT since SQLite doesn't have native UUID
    label TEXT NOT NULL,           -- Type of node (venue, event, artist, etc.)
    data TEXT NOT NULL,            -- JSON data for the node
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create edges table
-- This table stores relationships between nodes
CREATE TABLE IF NOT EXISTS edges (
    id TEXT PRIMARY KEY,           -- UUID as TEXT
    source_id TEXT NOT NULL,       -- Source node ID
    target_id TEXT NOT NULL,       -- Target node ID
    relation TEXT NOT NULL,        -- Type of relationship (performs_at, happening_on, etc.)
    data TEXT,                     -- Optional JSON data for the relationship
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    
    -- Foreign key constraints
    FOREIGN KEY (source_id) REFERENCES nodes(id) ON DELETE CASCADE,
    FOREIGN KEY (target_id) REFERENCES nodes(id) ON DELETE CASCADE
);

-- Create indexes for better query performance
CREATE INDEX IF NOT EXISTS idx_nodes_label ON nodes(label);
CREATE INDEX IF NOT EXISTS idx_edges_source_id ON edges(source_id);
CREATE INDEX IF NOT EXISTS idx_edges_target_id ON edges(target_id);
CREATE INDEX IF NOT EXISTS idx_edges_relation ON edges(relation);
CREATE INDEX IF NOT EXISTS idx_edges_source_target ON edges(source_id, target_id);

-- Create triggers to update the updated_at timestamp
CREATE TRIGGER IF NOT EXISTS nodes_updated_at 
    AFTER UPDATE ON nodes
BEGIN
    UPDATE nodes SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS edges_updated_at 
    AFTER UPDATE ON edges
BEGIN
    UPDATE edges SET updated_at = datetime('now') WHERE id = NEW.id;
END;
