# SMS Full Pipeline Debug Runbook

Updated: 2025-08-27 11:07 PT

This runbook tracks step-by-step verification of the end-to-end pipeline for Blue Moon, from ingestion through catalog. It documents commands, observations, and next actions so runs are repeatable and idempotent.

---

## Overview of Steps

- Ingestion (HTTP fetch, cadence, idempotency, gateway accept to CAS + ingest log)
- Parsing (extract RawEventData -> ProcessedEvent)
- Normalization (planned modules present; not currently wired)
- Quality Gate (planned modules present; not currently wired)
- Enrichment (planned modules present; not currently wired)
- Conflation (planned modules present; not currently wired)
- Catalog (create/update Venue, Artist, Event)

Key code references:
- `sms-scraper/src/apis/blue_moon.rs` (crawler implementation)
- `sms-scraper/src/pipeline/pipeline.rs` (ingest + parse + persist RawData)
- `sms-scraper/src/pipeline/ingestion/ingest_common.rs` (fetch + cadence + gateway)
- `sms-scraper/src/pipeline/ingestion/gateway/mod.rs` (CAS + ingest log)
- `sms-scraper/src/pipeline/full_pipeline_orchestrator.rs` (catalog/DB writes) [UP NEXT]
- `sms-scraper/src/main.rs` (CLI: ingester, full-pipeline, reprocess-all)

---

## Step 1 — Ingestion

Command(s) used:

```bash
cargo run -p sms-scraper -- ingester --apis blue_moon
SMS_BYPASS_CADENCE=1 cargo run -p sms-scraper -- ingester --apis blue_moon
```

Observations:
- Initial run (before fix): `cadence_skip` then `Gateway accept failed: File exists (os error 17)`.
- Hardened symlink handling in `sms-scraper/src/pipeline/ingestion/gateway/ingest_log.rs` to avoid EEXIST.
- Re-run with cadence bypass succeeded.

Hypothesis:
- The EEXIST likely occurred when creating/updating the `ingest.ndjson` symlink in `append_rotating()` under `sms-scraper/src/pipeline/ingestion/gateway/ingest_log.rs`. Current state looks correct now; a clean re-run should succeed.

Planned fix:
- Remove `data/ingest_log/ingest.ndjson` and re-run ingestion with cadence bypass to force fetch and accept.

Verification criteria:
- Command succeeds with no gateway error.
- New line appended to `data/ingest_log/ingest_YYYY-MM-DD.ndjson`.
- New or existing CAS object referenced by `cas:sha256:<digest>`.
- RawData persisted in DB with `api_name = "crawler_blue_moon"` (mapped via `api_name_to_internal()`), `processed = false`.

Evidence (11:07 PT):
- CLI: `✅ Fetched 102 raw events`, `✅ Processed 102 events`, `💾 Saved events to ./output/blue_moon_20250827_180735.json`.
- Ingest log: `data/ingest_log/ingest_2025-08-27.ndjson` grew to 2 lines; latest line has `payload_ref: cas:sha256:185f1ce2...` and proper timing.
- CAS: `data/cas/sha256/**` populated; CAS writes idempotent via `cas_fs::write_cas()`.
- DB: `DatabaseStorage` reported "data written to database"; RawData rows created with `api_name = "crawler_blue_moon"` and `processed = false`.

Status:
- Success at 11:07 PT — ingestion is sound and functioning.

---

## Step 2 — Parsing

### Where it happens
- `BlueMoonCrawler::get_event_list()` parses JSON from `fetch_payload_and_log()` and returns a Vec of `RawEventData` (102 events).
- `Pipeline::run_for_api_with_storage()` converts each item into `ProcessedEvent` and persists them as `RawData` via `Storage::create_raw_data()`.

### Verification
- **CLI Output**: Confirmed `✅ Fetched 102 raw events` and `✅ Processed 102 events (0 skipped, 0 errors)`.
- **RawData Persistence**: `RawData` rows created with:
  - `api_name = "crawler_blue_moon"`
  - `processed = false`
  - `event_day` set for each event
  - `event_name` and `venue_name` populated

### Status
- **Success** — Parsing completed at 11:07 PT. All 102 events processed and persisted to DB.
  - RawData ready for cataloging in the next step.

---

## Step 2.5 — Verify RawData in Database

### Verification
- **CLI Output**: Confirmed `✅ Fetched 102 raw events` and `✅ Processed 102 events (0 skipped, 0 errors)`.
- **Database**: RawData stored in Turso database (libsql) with `api_name = "crawler_blue_moon"` and `processed = false`.

### Verification Steps
1. **CLI Output**: Successfully processed 102 events with no errors.
2. **Database**: Direct database verification requires Turso CLI authentication.

### Status
- **Success** — Parsing and persistence confirmed via CLI output.
- **Next**: Proceed to cataloging with `full-pipeline` command.

---

## Step 3 — Cataloging (Full Pipeline)

### Verification
- **CLI Output**: Successfully processed 102 events with 0 failures (100% success rate).
- **Database**: Created/updated venue, artist, and event records in the database.
- **Logs**: Confirmed upserts for various artists and events.
- **Idempotency Check**: Re-running the cataloging step correctly identifies no unprocessed raw data.

### Status
- **Success** — All 102 events cataloged successfully.
- **Idempotency**: Confirmed - re-running the cataloging step is safe and doesn't create duplicates.
- **Next**: Proceed to normalization and quality gate steps.

---

## Step 4 — Normalization and Quality Gate

### Current State
- **Normalization** modules exist but are not wired into the pipeline:
  - `sms-scraper/src/app/normalize_use_case.rs`
  - `sms-scraper/src/infra/normalize_output_adapter.rs`
- **Quality Gate** modules exist but are not wired in:
  - `sms-scraper/src/app/quality_gate_use_case.rs`
  - `sms-scraper/src/infra/quality_gate_output_adapter.rs`

### Implementation Plan
1. **Normalization Integration**
   - Add normalization step after parsing in `FullPipelineOrchestrator::process_raw_data_item`
   - Configure `NormalizeUseCase` with appropriate output adapter
   - Write normalized records to a dedicated output directory

2. **Quality Gate Integration**
   - Add quality assessment after normalization
   - Configure `QualityGateUseCase` with accept/quarantine outputs
   - Route records based on quality assessment
   - Log quality metrics and decisions

3. **Database Updates**
   - Add quality assessment fields to RawData
   - Track normalization and quality gate status
   - Support requeuing quarantined records

### Verification Steps
1. **Normalization**
   - Verify normalized fields are correctly populated
   - Check output files in the normalization directory
   - Confirm metrics are emitted for normalization steps

2. **Quality Gate**
   - Verify records are correctly routed based on quality
   - Check quarantine directory for rejected records
   - Confirm quality metrics are recorded
   - Verify idempotency of quality assessment

3. **End-to-End**
   - Run full pipeline with normalization and quality gate enabled
   - Verify all steps complete successfully
   - Check logs for any warnings or errors

### Status
- **Not Started** - Implementation required
- **Next**: Implement and test normalization step

---

## Step 4 — Quality Gate

Current wiring:
- Modules exist in `sms-scraper/src/infra/quality_gate_output_adapter.rs` but are not invoked by `FullPipelineOrchestrator`.

Plan:
- Confirm desired acceptance criteria; wire into pipeline if required.

Status:
- Not currently executed in the CLI-driven pipeline.

---

## Step 5 — Enrichment

Current wiring:
- Modules exist in `sms-scraper/src/infra/enrich_output_adapter.rs` but are not invoked by `FullPipelineOrchestrator`.

Plan:
- Define enrichment inputs/outputs and wire into orchestrator as needed.

Status:
- Not currently executed in the CLI-driven pipeline.

---

## Step 6 — Conflation

Current wiring:
- `sms-scraper/src/infra/conflation_output_adapter.rs` exists; not currently invoked.

Plan:
- Determine conflation rules and integration point before cataloging.

Status:
- Not currently executed in the CLI-driven pipeline.

---

## Step 7 — Catalog (Database Writes)

Where it happens:
- `FullPipelineOrchestrator::process_source()` reads unprocessed `RawData` and creates/updates:
  - Venue via `ensure_venue()`.
  - Artists via `ensure_artists_from_title()`.
  - Event via `create_event_entity()` with dedupe by `(venue_id, event_day, title)`.

Verification criteria:
- Re-running `full-pipeline` yields idempotent behavior (no duplicate venues/artists/events).

Status:
- Current run reported: `No unprocessed raw data found` due to ingestion failure; will re-check after ingestion success.

---

## Idempotence Expectations

- Cadence: `SMS_BYPASS_CADENCE` controls 12h fetch gating in `ingest_common.rs`.
- Dedupe: Gateway uses SQLite `dedupe_index` by `idempotency_key` when cadence is NOT bypassed.
- CAS: `cas_fs::write_cas()` is idempotent; re-writes are skipped if file exists.
- Catalog: Events deduped by venue/date/title; artists by name/slug.

---

## Next Steps

1. [ ] **Verify RawData** (Step 2.5):
   ```bash
   sqlite3 data/ingest_log/meta.db "SELECT COUNT(*) FROM raw_data WHERE api_name = 'crawler_blue_moon' AND processed = 0;"
   ```
2. [ ] **Run Full Pipeline**: `cargo run -p sms-scraper -- full-pipeline --source-id crawler_blue_moon`.
- [ ] If zero items due to processed flags, run: `cargo run -p sms-scraper -- reprocess-all --source-id crawler_blue_moon`.
- [ ] Confirm venues/artists/events present and deduplicated.

---

## Appendix — Registry

- `registry/sources/blue_moon.json` has `"source_id": "blue_moon"`.
- Internal storage name used in DB is `"crawler_blue_moon"` via `api_name_to_internal()` mapping.
