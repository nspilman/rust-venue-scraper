# Warning Cleanup Checklist

This checklist captures current compiler warnings from `cargo build` and groups them by category. Use the checkboxes to track remediation.

Generated: 2025-08-20
Last Updated: 2025-08-20

✅ **ALL PRODUCTION BUILD WARNINGS HAVE BEEN RESOLVED!**

The production build (`cargo build --release`) and development build (`cargo build`) now compile with zero warnings.

## Legend
- [ ] Pending
- [x] Resolved

---

## 1) Unused import
- [x] src/pipeline/processing/catalog/handlers/venue.rs:13 — remove unused import `crate::pipeline::processing::normalize::NormalizedEntity`

Suggested actions:
- Remove import (quick fix), or
- Start using the type if intended.

---

## 2) Dead code: never constructed types
- [x] src/pipeline/processing/catalog/idempotency.rs:4 — struct `IdempotencyChecker`
- [x] src/pipeline/processing/catalog/provenance.rs:10 — struct `ProvenanceTracker`
- [x] src/app/enrich_use_case.rs:8 — struct `EnrichUseCase`
- [x] src/app/conflation_use_case.rs:228 — struct `ConflationStats`

Suggested actions:
- Wire into code paths or tests, or
- Gate with feature flags (`#[cfg(feature = "...")]`), or
- Annotate with `#[allow(dead_code)]`, or
- Remove if not needed.

---

## 3) Unused associated functions and methods
- catalog/idempotency.rs
  - [x] `event_has_changes`
  - [x] `venue_has_changes`
  - [x] `artist_has_changes`
- catalog/provenance.rs
  - [x] `new`
  - [x] `set_process_run_id`
  - [x] `emit_record`
  - [x] `venue_created`
  - [x] `venue_matched`
  - [x] `venue_duplicate`
  - [x] `venue_uncertain`
  - [x] `event_created`
  - [x] `event_updated`
  - [x] `event_duplicate`
  - [x] `event_uncertain`
  - [x] `artist_created`
  - [x] `artist_matched`
  - [x] `artist_uncertain`
- observability/metrics.rs
  - [x] `batch_processed` (annotated with #[allow(dead_code)])
  - [x] `output_failed` (removed - was unused)
  - [x] `processing_duration` (removed - was unused)
  - [x] `MetricName::ConflationOutputFailed` (removed - was unused)
  - [x] `MetricName::ConflationProcessingDuration` (removed - was unused)
  - [x] `MetricsQualityGate::new` (made test-only with #[cfg(test)])
- pipeline/processing/normalize/normalizers/base.rs
  - [x] `VenueStateManager::reset` (made test-only with #[cfg(test)])
  - [x] `ArtistStateManager::reset` (made test-only with #[cfg(test)])
- pipeline/processing/normalize/registry.rs
  - [x] `NormalizationRegistry::register` (deleted)
  - [x] `NormalizationRegistry::list_sources` (made test-only with #[cfg(test)])
- pipeline/processing/quality_gate/mod.rs
  - [x] `DefaultQualityGate::with_config` (deleted)
- pipeline/processing/enrich.rs
  - [x] `MetricsEnricher::new` (deleted)
  - [x] `DefaultEnricher::with_city_center` (deleted)
- pipeline/processing/conflation.rs
  - [x] `DefaultConflator::with_config` (deleted)
  - [x] `DefaultConflator::add_canonical_entity` (deleted)
- pipeline/processing/catalog/catalogger.rs
  - [x] `Catalogger::with_registry` (made test-only with #[cfg(test)])
  - [x] `Catalogger::supported_entity_types` (made test-only with #[cfg(test)])
- pipeline/processing/catalog/handlers/artist.rs
  - [x] `ArtistHandler::new` (made test-only with #[cfg(test)])
- pipeline/processing/catalog/handlers/event.rs
  - [x] `EventHandler::new` (made test-only with #[cfg(test)])
- pipeline/processing/catalog/handlers/venue.rs
  - [x] `VenueHandler::new` (made test-only with #[cfg(test)])
- pipeline/processing/catalog/registry.rs
  - [x] `EntityRegistry::registered_types` (made test-only with #[cfg(test)])
- app/ports.rs
  - [x] `EnrichOutputPort::write_enriched_record` (annotated with #[allow(dead_code)])
  - [x] `QualityGateOutputPort::write_quality_assessed_record` (annotated with #[allow(dead_code)])
- app/quality_gate_use_case.rs
  - [x] `QualityGateUseCase::new` (module made test-only via #[cfg(test)])
  - [x] `QualityGateUseCase::get_batch_stats` (module made test-only via #[cfg(test)])
  - [x] `QualityGateBatchStats::acceptance_rate` (module made test-only via #[cfg(test)])
  - [x] `QualityGateBatchStats::quarantine_rate` (module made test-only via #[cfg(test)])
- app/enrich_use_case.rs
  - [x] `EnrichUseCase::new` (module made test-only via #[cfg(test)])
  - [x] `EnrichUseCase::with_default_enricher` (module made test-only via #[cfg(test)])
  - [x] `EnrichUseCase::enrich_record` (module made test-only via #[cfg(test)])
  - [x] `EnrichUseCase::enrich_batch` (module made test-only via #[cfg(test)])
  - [x] `EnrichUseCase::get_batch_stats` (module made test-only via #[cfg(test)])
  - [x] `EnrichBatchStats` helper methods (module made test-only via #[cfg(test)])
- app/conflation_use_case.rs
  - [x] `ConflationUseCase::with_conflator` (module made test-only via #[cfg(test)])
  - [x] `ConflationUseCase::conflate_batch` (module made test-only via #[cfg(test)])
  - [x] `ConflationUseCase::get_conflation_stats` (module made test-only via #[cfg(test)])
- infra/enrich_output_adapter.rs
  - [x] `FileEnrichOutputAdapter` (entire struct removed - was unused)
- infra/quality_gate_output_adapter.rs
  - [x] `FileQualityGateOutputAdapter` (entire struct removed - was unused)
- infra/conflation_output_adapter.rs
  - [x] `ConflationOutputAdapter::with_partitioning` (removed - was unused)

Suggested actions:
- Add minimal call sites (e.g., unit tests) if these are intended APIs, or
- Gate with features/annotate as allowed dead code, or
- Remove if not needed.

---

## 4) Never read fields
- pipeline/processing/conflation.rs
  - [x] `ConflationConfig.enabled_strategies`
  - [x] `ConflationConfig.text_similarity_threshold`
  - [x] `PotentialMatch.similarity_breakdown`
  - [x] `PotentialMatch.matched_record`
- pipeline/processing/catalog/candidate.rs
  - [x] `CatalogCandidate.entity_type` (removed - was unused)
  - [x] `CatalogCandidate.canonical_id` (removed - was unused) 
  - [x] `CatalogCandidate.prepared_at` (removed - was unused)
  - [x] `CatalogCandidate.entity_id()` (removed - was unused)
  - [x] `PersistedEntity::Venue(Venue)` payload is never read (converted to unit variant)
  - [x] `PersistedEntity::Event(Event)` payload is never read (converted to unit variant)
  - [x] `PersistedEntity::Artist(Artist)` payload is never read (converted to unit variant)
- app/quality_gate_use_case.rs (QualityGateBatchStats)
  - [ ] `total_records`, `accepted_count`, `accepted_with_warnings_count`, `quarantined_count`,
        `min_quality_score`, `max_quality_score`, `avg_quality_score`, `info_issues`,
        `warning_issues`, `error_issues`, `critical_issues`
- app/enrich_use_case.rs (EnrichBatchStats)
  - [ ] `total_records`, `spatially_binned`, `city_identified`, `with_coordinates`,
        `total_tags`, `total_warnings`, `min_confidence`, `max_confidence`,
        `avg_confidence`, `avg_tags_per_record`
- infra/{quality_gate_output_adapter,enrich_output_adapter}.rs
  - [ ] `File*OutputAdapter.file_path`

Suggested actions:
- Use the fields in logic or logging, or
- If intentionally unused for now, add `#[allow(dead_code)]` on the specific fields, or
- Refactor out until needed.

---

## 5) Enum variant payloads never read (compiler hint provided)
- pipeline/processing/catalog/candidate.rs
  - [x] `PersistedEntity::Venue(Venue)` (converted to unit variant)
  - [x] `PersistedEntity::Event(Event)` (converted to unit variant)
  - [x] `PersistedEntity::Artist(Artist)` (converted to unit variant)

Suggested actions:
- If you don’t need the payloads yet, convert to unit-like variants (e.g., `Venue`, `Event`, `Artist`) to silence warnings, or
- Start reading/using the payloads where appropriate (e.g., for diffing/logging).

---

## Quick global suggestions
- Run selective fixes: `cargo fix --lib -p sms_scraper` for obvious unused imports.
- Prefer feature-gating or targeted `#[allow(dead_code)]` over global allows.
- Add small unit tests to establish usage for intended APIs.

