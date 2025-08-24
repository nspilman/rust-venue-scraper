# Persistence design for SQLite (nodes + edges) with blob staging

This document outlines a persistence design that fits your current repository:
- Blob/object storage for ingestion and stage snapshots (append-only NDJSON)
- Final catalog persisted to SQLite (or libsql/Turso) using two tables: nodes and edges
- Deterministic idempotency, reproducible runs, and provenance

It aligns with the code you have today while tightening idempotency and wiring missing stage outputs.


## 1) Current state (observed in repo)

- Ingestion
  - Raw payloads are stored as content-addressable blobs (CAS) to local FS under `data/cas/sha256/...` or Supabase Storage when configured
    - Files: `src/pipeline/ingestion/gateway/{cas_fs.rs, cas_supabase.rs}`
  - Envelopes are appended to daily NDJSON logs under `data/ingest_log`; dedupe index, cadence markers, and consumer offsets stored in SQLite at `data/ingest_log/meta.db`
    - Files: `src/pipeline/ingestion/gateway/ingest_log.rs`, `src/pipeline/ingestion/ingest_meta.rs`
- Stage persistence
  - Normalize/Quality/Enrich ports exist; Normalize adapter is stubbed (logs only), Quality/Enrich adapters not yet implemented to write NDJSON
  - Conflation writes NDJSON via `ConflationOutputAdapter` (date-partitioned)
    - Files: `src/app/*_use_case.rs`, `src/app/ports.rs`, `src/infra/normalize_output_adapter.rs` (stub), `src/infra/conflation_output_adapter.rs` (real)
- Catalog (SQLite/libsql)
  - Schema: `migrations/001_create_nodes_and_edges.sql` (nodes id/label/data, edges id/source_id/target_id/relation/data)
  - Database access: `src/db.rs` (libsql) and `src/pipeline/storage/database.rs` (Storage impl)
  - Handlers: map conflated records into domain structs and call `Storage` methods; edges are `hosts` (venue→event) and `performs_at` (artist→event)

Gaps vs goal
- No blob stage persistence for Normalize/Quality/Enrich (only ingestion and conflation write NDJSON)
- Upserts use `INSERT OR REPLACE` keyed on `id`, which can delete+insert and ignore natural keys
- No unique constraints for dedupe keys (venue slug, artist slug, event key) or for edges (src_id, dst_id, relation)
- `Storage.create_*` generates new UUIDs even when conflation provided an id, breaking canonical mapping
- Event handler can fall back to using the event’s own id as a venue id (incorrect in rare paths)
- ExternalId nodes and provenance edges not modeled in the graph (though ProcessRun/ProcessRecord exist separately)


## 2) Target architecture (SQLite two-table friendly)

- Blob/object storage
  - Persist stage snapshots as gzipped NDJSON with a consistent envelope, enabling audit and reprocessing without re-running upstream stages
  - Recommended prefixes:
    - `raw/{source}/{YYYY}/{MM}/{DD}/{run_id}/{fetch_id}.json.gz`
    - `parsed/{source}/{run_id}/{parser_version}/{partition}.ndjson.gz`
    - `normalized/{source}/{run_id}/{normalizer_version}/{partition}.ndjson.gz`
    - `quality/{source}/{run_id}/{rule_version}/{partition}.ndjson.gz` (accepted and quarantined partitions)
    - `enriched/{source}/{run_id}/{enricher_version}/{partition}.ndjson.gz`
    - `conflation/{source}/{run_id}/{conflator_version}/{partition}.ndjson.gz`
  - Envelope fields to include in each record:
    - `run_id`, `code_version`, `stage_version`, `schema_version`
    - `source`, `source_record_id`, `url` (if available)
    - `collected_at`, `processed_at`, `content_hash`, `artifact_uri` (backref to previous stage)

- SQLite nodes/edges catalog
  - Keep tables as-is (nodes, edges) and leverage JSON1 expression indexes for uniqueness and idempotency
  - Entities (nodes.label): `venue`, `artist`, `event`, `external_id` (recommended for source linkage)
  - Edges (edges.relation): `hosts` (event at venue), `performs_at` (artist performs at event), `identified_by` (entity identified by external id), `merged_into` (duplicate resolution)

- Idempotency and upserts
  - Respect provided ids from conflation decisions
  - Use `INSERT ... ON CONFLICT(id) DO UPDATE` for nodes/edges (avoid `OR REPLACE`)
  - Enforce uniqueness:
    - Edges: unique `(source_id, target_id, relation)`
    - Venues: unique `(label='venue', lower(json_extract(data,'$.slug')))`
    - Artists: unique `(label='artist', lower(json_extract(data,'$.name_slug')))`
    - Events: unique `(label='event', lower(json_extract(data,'$.title')) || '|' || json_extract(data,'$.venue_id') || '|' || json_extract(data,'$.event_day'))`
      - Alternatively store a computed `dedupe_key` in `data` and index on that

- External IDs and provenance
  - Model `external_id` as nodes with `data.key = "{source}:{source_record_id}"`; unique on `(label='external_id', lower(json_extract(data,'$.key')))`
  - Add `identified_by` edges from entity nodes → external_id nodes; include `confidence`, `reasons`, and `artifact_uri` in edge `data`
  - Keep using `ProcessRun` and `ProcessRecord` nodes for audit; optionally add `produced_by` edges from entity nodes to the current `process_run`


## 3) SQLite DDL additions (new migration)

Create `migrations/002_indexes_and_pragmas.sql` with:

```sql
-- Enable foreign keys (SQLite local; libsql/Turso may enforce separately)
PRAGMA foreign_keys = ON;

-- Edge uniqueness: avoid duplicate relationships
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
-- If you prefer a precomputed key, replace this with '$.dedupe_key'
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
```

Notes
- Keep `migrations/001_create_nodes_and_edges.sql` unchanged (no schema changes required)
- We avoid triggers for `updated_at` because we won’t use REPLACE; instead we set timestamps explicitly on UPSERT


## 4) Upsert patterns (nodes and edges)

Node upsert using id (respect existing id):

```sql
-- Parameters: :id, :label, :data_json, :now
INSERT INTO nodes (id, label, data, created_at, updated_at)
VALUES (:id, :label, :data_json, COALESCE((SELECT created_at FROM nodes WHERE id = :id), :now), :now)
ON CONFLICT(id) DO UPDATE SET
  data = excluded.data,
  updated_at = excluded.updated_at;
```

Edge upsert with unique (src, dst, relation):

```sql
-- Parameters: :id, :src, :dst, :rel, :data_json, :now
INSERT INTO edges (id, source_id, target_id, relation, data, created_at, updated_at)
VALUES (:id, :src, :dst, :rel, :data_json, COALESCE((SELECT created_at FROM edges WHERE id = :id), :now), :now)
ON CONFLICT(source_id, target_id, relation) DO UPDATE SET
  data = excluded.data,
  updated_at = excluded.updated_at;
```

Entity-level idempotency via JSON1 indexes:
- On persist, populate entity JSON fields consistently:
  - Venue: `slug`
  - Artist: `name_slug`
  - Event: ensure `title`, `event_day` (YYYY-MM-DD), and `venue_id` are present; optionally include `start_time` and/or a `dedupe_key`
- Reads do not need to scan all nodes—SQLite can use the indexes


## 5) Stage persistence to blobs (aligning with existing ports)

- NormalizeOutputPort → new file adapter (NDJSON writer)
  - Path: `normalized/{source}/{YYYY}/{MM}/{DD}/normalized-<entity>-<date>.ndjson`
  - Write the whole `NormalizedRecord` envelope line-by-line
- QualityGateOutputPort → two file adapters or one with modes
  - Paths: `quality/accepted/...` and `quality/quarantined/...`
  - Include decision and reasons in the envelope
- EnrichOutputPort → new file adapter (NDJSON)
  - Path: `enriched/{source}/{YYYY}/{MM}/{DD}/enriched-<entity>-<date>.ndjson`
- ConflationOutputPort → already implemented; keep as-is

General
- Use a consistent line-oriented format and optional gzip for larger batches
- Include `run_id`, `stage_version`, `artifact_uri` to prior stage for lineage


## 6) Catalog write flow (from conflation)

For each `ConflatedRecord`:
1) ExternalId: upsert `external_id` node with `data.key = "{source}:{source_record_id}"`; update `first_seen_at`/`last_seen_at` fields in `data`
2) Entity node (venue/artist/event):
   - Respect `id` if provided by conflation; otherwise generate once and reuse
   - Upsert node JSON with `content_hash` and provenance fields (e.g., last_updated_by_run, artifact_uri)
3) IDENTIFIED_BY edge: upsert entity → external_id with props `{ confidence, reasons, artifact_uri, decided_at }`
4) Event relations:
   - Upsert `hosts` edge: `event -> venue`
   - Upsert `performs_at` edges: `artist -> event`

Idempotency
- Re-running the same input should be no-op unless the entity JSON `content_hash` changes


## 7) Implementation plan (file-by-file)

This plan keeps your two-table design and avoids schema changes—only adds indexes and PRAGMA.

A. Migrations
- [ ] Add `migrations/002_indexes_and_pragmas.sql` with the DDL in section 3
- [ ] Ensure migrations run early (`DatabaseManager::run_migrations` already applies all files by include_str—you may need to adjust to apply files in order or incorporate the new SQL similarly)

B. Storage upserts and id handling
- [ ] Update `src/pipeline/storage/database.rs`
  - Respect existing ids in `create_venue/create_artist/create_event`: only generate when `id` is `None`
  - Replace `INSERT OR REPLACE` with `INSERT ... ON CONFLICT(id) DO UPDATE` for nodes
  - In `create_edge`, use `ON CONFLICT(source_id, target_id, relation) DO UPDATE`
  - Ensure timestamps are set explicitly; REPLACE will no longer be used
- [ ] Consider adding helper(s) for JSON merge if you need to preserve parts of existing `data` (optional now; last-write-wins is acceptable to start)

C. Event venue resolution correctness
- [ ] In `src/pipeline/processing/catalog/handlers/event.rs`, ensure `venue_id` is resolved:
  - Resolve venue by slug (or precomputed key) via a short lookup (consider a small helper in `Storage` or query by JSON index once added)
  - Never fall back to the event’s canonical id as a venue id

D. Stage blob adapters
- [ ] Implement real NDJSON writers:
  - `src/infra/normalize_output_adapter.rs` — replace log-only with actual append to date-partitioned NDJSON, mirroring `ConflationOutputAdapter`
  - Add `src/infra/quality_output_adapter.rs` for `QualityGateOutputPort` (accepted/quarantined)
  - Add `src/infra/enrich_output_adapter.rs` for `EnrichOutputPort`
- [ ] Wire use cases to these adapters where used (e.g., in `main.rs` demo flows or pipelines)

E. ExternalId modeling (optional but recommended)
- [ ] Add helpers in `DatabaseStorage`:
  - `get_or_create_external_id(source: &str, source_record_id: &str) -> (external_id_node_id, was_created)`
  - Upsert `identified_by` edges with confidence/reasons/artifact_uri
- [ ] Update handlers (or `Catalogger`) to call these helpers during persist

F. Metrics and observability
- [ ] Add per-stage persistence metrics: `blob_write_success/error/bytes`, `db_upsert_success/error/rows`, `idempotent_conflicts`, `stage_latency`
- [ ] Include `run_id`, `source`, `source_record_id` as trace attributes on writes

G. Tests and validation
- [ ] Unit tests for adapters (similar to conflation adapter tests)
- [ ] Storage tests: id-respecting creates, ON CONFLICT updates, edge uniqueness
- [ ] End-to-end smoke: run parse→normalize→quality→enrich→conflate (write NDJSON) → catalog (read conflation) and verify nodes/edges

H. Rollout
- [ ] Local: enable WAL and busy_timeout for local SQLite if you run direct (libsql/Turso may differ)
- [ ] Backups: use SQLite `.backup` API or Turso snapshot as appropriate
- [ ] Lifecycle: configure blob retention (e.g., keep raw indefinitely, intermediate 90–180 days)


## 8) PRAGMA and operational guidance

If running local SQLite directly (not via libsql):
- `PRAGMA journal_mode=WAL;`
- `PRAGMA synchronous=NORMAL;` (or FULL for stronger durability)
- `PRAGMA foreign_keys=ON;`
- `PRAGMA busy_timeout=5000;`

With libsql/Turso, some PRAGMAs may be managed by the service—focus on correctness via indexes and upsert semantics.


## 9) Tradeoffs

- Keeping stage data in blobs avoids load on SQLite’s single-writer path and gives full auditability
- JSON1 expression indexes are slightly slower than dedicated columns but preserve your two-table simplicity
- ExternalId as nodes increases relationship clarity and queryability without extra tables


## 10) Quick reference (snippets)

Blob envelope sketch (per record line):
```json
{
  "run_id": "<uuid>",
  "stage_version": "<git-sha>",
  "source": "kexp",
  "source_record_id": "<id-from-source>",
  "artifact_uri": "cas:sha256:<hex>",
  "processed_at": "2025-08-22T15:00:00Z",
  "payload": { /* Normalized/Quality/Enriched/Conflation content */ }
}
```

Edge upsert (unique on src/dst/relation):
```sql
INSERT INTO edges (id, source_id, target_id, relation, data, created_at, updated_at)
VALUES (:id, :src, :dst, :rel, :data_json, :now, :now)
ON CONFLICT(source_id, target_id, relation) DO UPDATE SET
  data = excluded.data,
  updated_at = excluded.updated_at;
```

Event uniqueness (expression index):
```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_nodes_event_key
  ON nodes(
    label,
    lower(json_extract(data, '$.title')) || '|' ||
    json_extract(data, '$.venue_id') || '|' ||
    json_extract(data, '$.event_day')
  ) WHERE label = 'event';
```


---

This plan keeps your existing schema, adds safe idempotency, and wires stage persistence using adapters you already designed. It prioritizes correctness of ids, dedupe via indexes, and reproducibility through blob snapshots.

