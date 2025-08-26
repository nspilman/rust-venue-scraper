-- PRAGMAs for local SQLite (libsql/Turso may ignore)
PRAGMA foreign_keys = ON;

-- Ensure unique relationships: prevent duplicate edges of the same kind between two nodes
CREATE UNIQUE INDEX IF NOT EXISTS idx_edges_src_dst_rel
  ON edges(source_id, target_id, relation);

-- Venue uniqueness by slug (stored in node JSON data)
CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_venue_slug
  ON nodes(
    label,
    lower(json_extract(data, '$.slug'))
  ) WHERE label = 'venue';

-- Artist uniqueness by name_slug
CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_artist_slug
  ON nodes(
    label,
    lower(json_extract(data, '$.name_slug'))
  ) WHERE label = 'artist';

-- Event uniqueness by composite key (title+venue+date)
-- If you later store a precomputed dedupe_key, switch to that field
CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_event_key
  ON nodes(
    label,
    lower(json_extract(data, '$.title')) || '|' ||
    json_extract(data, '$.venue_id') || '|' ||
    json_extract(data, '$.event_day')
  ) WHERE label = 'event';

-- ExternalId uniqueness by key = "{source}:{source_record_id}"
CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_external_id_key
  ON nodes(
    label,
    lower(json_extract(data, '$.key'))
  ) WHERE label = 'external_id';

