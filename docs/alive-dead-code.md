# Alive vs Dead Code Paths (SMS Scraper Pipeline)

This document enumerates the code paths that are executed by the current pipeline run versus legacy/unused paths, based on the repository’s binaries, CLI, and Docker configuration.

## Summary

- The production/default pipeline entrypoint is the `sms-scraper` CLI.
- Docker runs `Ingester` first, then `FullPipeline` per source.
- The path exercised in Docker uses `FullPipelineOrchestrator` (legacy/compat orchestration) rather than the newer modular `PipelineOrchestrator`.
- Step-specific CLI subcommands (`Parse`, `Normalize`, `QualityGate`, `Enrich`, `Conflation`, `Catalog`) are available but not used in the default Docker run.

## Entrypoints

- Alive: `sms-scraper/src/main.rs` — primary CLI.
  - Commands used by Docker:
    - `Ingester { apis, bypass_cadence }`
    - `FullPipeline { source_id, bypass_cadence }`
  - Commands available but not used by Docker (manually runnable):
    - `ModularPipeline { source_id, parse_only, ingestion_only }`
    - `Parse`, `Normalize`, `QualityGate`, `Enrich`, `Conflation`, `Catalog`

- Secondary/demo binaries (not used in Docker):
  - `src/bin/demo-full-pipeline.rs` — full legacy pipeline demo.
  - Other `src/bin/*` utilities (diagnostics, dashboards, etc.).

## Orchestration Layers

- Alive (default path): `sms-scraper/src/pipeline/full_pipeline_orchestrator.rs`
  - `FullPipelineOrchestrator::new()` — constructs DB storage, loads `SourceRegistry`.
  - `FullPipelineOrchestrator::process_source()` — core pipeline driver used by `FullPipeline`.
    - Fetches unprocessed raw data; conditionally calls ingestion if none or forced.
    - Iterates raw data and runs: `parse_raw_data()` → `normalize_parsed_data()` → `quality_gate_check()` → `enrich_data()` → `conflate_entities()` → `catalog_entities()`.
    - Note: several helpers are marked DEPRECATED in favor of modular steps, but are still executed by this orchestrator.
  - `FullPipelineOrchestrator::run_ingestion_for_source()` — invoked by both `Ingester` command and from `process_source()` when needed. Uses `steps::IngestionStep`.
  - CLI subcommands `Parse/Normalize/...` call the corresponding `run_*_for_source()` adapters, which delegate to `steps::*` implementations.

- Available (not used by Docker): `sms-scraper/src/pipeline/orchestrator.rs`
  - `PipelineOrchestrator` + `PipelineConfig` — newer modular pipeline orchestration used by `ModularPipeline` command.

## Step Implementations

Directory: `sms-scraper/src/pipeline/steps/`

- Alive (via default Docker flow)
  - `ingestion.rs` — used indirectly by `FullPipelineOrchestrator::run_ingestion_for_source()`.
- Alive (CLI-only; not executed in Docker’s default run)
  - `parse.rs`, `normalize.rs`, `quality_gate.rs`, `enrich.rs`, `conflation.rs`, `catalog.rs` — executed when using their dedicated CLI subcommands or the `ModularPipeline` orchestration.

## Parsers / Source Registry

- Alive: `sms-scraper/src/registry/source_loader.rs` (loaded by `FullPipelineOrchestrator::new()`).
- Alive: Parser factory usage in `FullPipelineOrchestrator::parse_raw_data()`:
  - `super::super::apis::factory::create_parser(api_name)` — instantiates per-source parsers used in default flow.

## Storage Layer

- Alive: `sms_core` storage (database-backed) is constructed in both `sms-scraper/src/main.rs` and `FullPipelineOrchestrator::new()` and exercised by all pipeline operations.

## Docker Path (authoritative runtime)

File: `sms-scraper/Dockerfile`

- CMD executed in container:
  - `sms-scraper ingester --apis blue_moon,sea_monster,darrells_tavern,barboza,kexp,neumos,conor_byrne --bypass-cadence`
  - Sequential `sms-scraper full-pipeline --source-id <source>` for each source

Therefore, when the pipeline runs via Docker:

- Alive code paths:
  - `sms-scraper/src/main.rs` → `Commands::Ingester` → `FullPipelineOrchestrator::run_ingestion_for_source()` → `steps::IngestionStep`.
  - `sms-scraper/src/main.rs` → `Commands::FullPipeline` → `FullPipelineOrchestrator::process_source()` → in-orchestrator steps (`parse_raw_data`, `normalize_parsed_data`, `quality_gate_check`, `enrich_data`, `conflate_entities`, `catalog_entities`).
- Not executed in Docker by default:
  - `Commands::{Parse, Normalize, QualityGate, Enrich, Conflation, Catalog}` (these call `run_*_for_source()` which delegate to step files).
  - `Commands::ModularPipeline` (uses `PipelineOrchestrator`).

## Notable Architectural Violation (Alive)

- The current default flow acknowledges that “the ingester already did the parsing work” inside `FullPipelineOrchestrator` comments and logic. This violates the “Platonic Ideal” where ingestion should only fetch raw bytes, and parsing should be a separate pipeline stage.
- Practically, this means some ingesters may store structured JSON rather than raw bytes, and `parse_raw_data()` must handle both already-structured and raw inputs.

## Legacy / Likely Dead or Secondary

- Legacy processing path: `sms-scraper/src/pipeline/processing/` (marked as “legacy processing module for backward compatibility” in `pipeline/mod.rs`). No Docker/CLI code directly routes here in the default run.
- `steps/*` are not dead, but are “secondary alive”: used by the CLI’s per-step commands and by the modular orchestrator; not exercised by the Docker default.
- Demo binary `src/bin/demo-full-pipeline.rs` is not used by Docker; keep as reference/demo unless you run it explicitly.

## Recommended Actions

- Keep: `sms-scraper/src/main.rs`, `pipeline/full_pipeline_orchestrator.rs`, `steps/ingestion.rs`, source parsers, `sms_core` storage.
- Keep (secondary): `steps/*`, `orchestrator.rs`, `pipeline_config.rs` to support `ModularPipeline` and per-step commands.
- Review for removal/archival:
  - `pipeline/processing/` legacy module if no longer routed anywhere practical.
  - Demo binaries under `src/bin/*` not used in CI or ops.
- Plan to fix architecture violation:
  - Ensure ingestion stores only raw HTML/JSON bytes with `processed=false`.
  - Move all parsing to the parse step invoked by the orchestrator (either modular or update the full pipeline to call `steps::ParseStep`).

## How this was determined

- CLI wiring in `sms-scraper/src/main.rs`.
- Orchestration in `sms-scraper/src/pipeline/full_pipeline_orchestrator.rs` and `sms-scraper/src/pipeline/mod.rs`.
- Runtime behavior in `sms-scraper/Dockerfile` CMD.

If you want, I can add a script to quickly verify “alive” code by running `cargo test` with targeted integration smoke tests and capturing callsite logs for the above functions.
