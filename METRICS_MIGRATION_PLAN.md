# Metrics Migration Implementation Plan

## Current State Audit

### 🔍 What We Have Now

#### 1. **Metric Definition Structure**
- **Location**: Separate files per phase in `src/metrics/`
  - `sources.rs` - 7 metrics defined
  - `gateway.rs` - 10 metrics defined  
  - `ingest_log.rs` - 11 metrics defined
  - `parser.rs` - 15 metrics defined
- **Pattern**: Using `phase_metric!` macro for naming consistency
- **Registration**: Each phase implements `PhaseMetrics` trait with `register_metrics()`

#### 2. **Metric Usage Patterns**

##### Direct String-Based Recording
```rust
// Current pattern in sources.rs
::metrics::counter!(phase_metric!(counter, "sources", "requests_success")).increment(1);

// Current pattern in gateway.rs
::metrics::histogram!(phase_metric!(histogram, "gateway", "payload_bytes"))
    .record(payload_bytes as f64);
```

##### Phase-Specific Static Methods
```rust
// In application code (ingest_common.rs)
SourcesMetrics::record_registry_load_success(source_id);
GatewayMetrics::record_envelope_accepted(&source_id, payload.len(), duration);
IngestLogMetrics::record_write_success(payload.len());
```

#### 3. **Current Problems**
- ❌ Metric names constructed at runtime via macros
- ❌ No compile-time verification of metric names
- ❌ Dashboard generation requires hardcoding metric names
- ❌ Metadata (units, labels, help text) scattered across implementations
- ❌ No connection between metric definitions and dashboard queries

### 📊 Metrics Inventory

| Phase | Count | Key Metrics |
|-------|-------|-------------|
| **Sources** | 7 | requests_success, requests_error, request_duration_seconds, payload_bytes, registry_loads, cadence_checks |
| **Gateway** | 10 | envelopes_accepted, envelopes_deduplicated, cas_writes_success/error, processing_duration_seconds, payload_bytes |
| **Ingest Log** | 11 | writes_success/error, write_bytes, consumer_reads, current_file_bytes, consumer_lag_bytes |
| **Parser** | 15 | parse_success/error, parse_duration_seconds, records_extracted, batch_size |

**Total**: 43 metrics across 4 phases

## Implementation Steps

### Phase 1: Create Metric Catalog (2-3 hours)

#### Step 1.1: Add Dependencies
```toml
# Cargo.toml
[dependencies]
once_cell = "1.19"
```

#### Step 1.2: Create Catalog Structure
```
src/metrics/
├── catalog/
│   ├── mod.rs           # Main catalog module
│   ├── definitions.rs   # All metric definitions
│   ├── keys.rs          # Type-safe metric keys
│   └── types.rs         # Shared types (MetricType, Unit, etc.)
```

#### Step 1.3: Define All Existing Metrics
- Convert each metric from current phase modules into catalog entries
- Preserve all existing metadata (labels, help text)
- Add dashboard configuration for each metric

#### Step 1.4: Create Type-Safe Keys
```rust
pub struct MetricKeys;
impl MetricKeys {
    // Sources
    pub const SOURCES_REQUESTS_SUCCESS: &'static str = "sources.requests.success";
    pub const SOURCES_REQUESTS_ERROR: &'static str = "sources.requests.error";
    // ... all 43 metrics
}
```

### Phase 2: Create Unified Recorder (1-2 hours)

#### Step 2.1: Implement MetricRecorder
- Create `src/metrics/recorder.rs`
- Implement `init()` to register all metrics from catalog
- Implement `record()` and `increment()` methods
- Add label validation

#### Step 2.2: Update Initialization
- Modify `src/metrics/lib.rs` to use catalog for registration
- Update `register_all_metrics()` to iterate catalog

### Phase 3: Migrate Runtime Code (3-4 hours)

#### Step 3.1: Update Phase Modules
Transform each phase module to use the catalog:

**Before** (sources.rs):
```rust
pub fn record_request_success(_source_id: &str, duration_secs: f64, payload_bytes: usize) {
    ::metrics::counter!(phase_metric!(counter, "sources", "requests_success")).increment(1);
    ::metrics::histogram!(phase_metric!(histogram, "sources", "request_duration_seconds"))
        .record(duration_secs);
}
```

**After**:
```rust
pub fn record_request_success(source_id: &str, duration_secs: f64, payload_bytes: usize) {
    MetricRecorder::increment(
        MetricKeys::SOURCES_REQUESTS_SUCCESS,
        &[("source_id", source_id)]
    );
    MetricRecorder::record(
        MetricKeys::SOURCES_REQUEST_DURATION,
        duration_secs,
        &[("source_id", source_id)]
    );
}
```

#### Step 3.2: Update Application Code
No changes needed! Application code already uses the phase-specific methods:
```rust
// This stays the same
SourcesMetrics::record_request_success(&spec.source_id, dur, payload.len());
```

### Phase 4: Implement Dashboard Builder (2-3 hours)

#### Step 4.1: Create Dashboard Module
- Create `src/metrics/dashboard.rs`
- Implement `DashboardBuilder::from_catalog()`
- Add panel generation for each metric type
- Group metrics by phase automatically

#### Step 4.2: Create Dashboard Binary
- Update `src/bin/build-dashboard.rs`
- Use catalog to generate complete dashboard
- Output valid Grafana JSON

### Phase 5: Testing & Validation (1-2 hours)

#### Step 5.1: Add Catalog Tests
```rust
#[test]
fn all_metrics_have_unique_names() { ... }

#[test]
fn all_metric_keys_exist_in_catalog() { ... }

#[test]
fn all_metrics_have_dashboard_config() { ... }
```

#### Step 5.2: Integration Testing
- Verify metrics still record correctly
- Test dashboard generation
- Validate Prometheus scraping

### Phase 6: Cleanup (1 hour)

#### Step 6.1: Remove Old Code
- Remove `phase_metric!` macro usage
- Clean up duplicate definitions
- Update documentation

#### Step 6.2: Update Documentation
- Update `src/metrics/README.md`
- Add catalog documentation
- Update usage examples

## Migration Checklist

### Pre-Migration
- [ ] Back up current dashboards
- [ ] Document current metric names
- [ ] Review all metric usage sites

### Phase 1: Catalog
- [ ] Add `once_cell` dependency
- [ ] Create catalog structure
- [ ] Define all 43 metrics
- [ ] Create type-safe keys
- [ ] Write catalog tests

### Phase 2: Recorder
- [ ] Implement MetricRecorder
- [ ] Update initialization
- [ ] Test metric registration

### Phase 3: Runtime
- [ ] Update sources.rs (7 metrics)
- [ ] Update gateway.rs (10 metrics)
- [ ] Update ingest_log.rs (11 metrics)
- [ ] Update parser.rs (15 metrics)
- [ ] Verify application code still works

### Phase 4: Dashboard
- [ ] Implement dashboard builder
- [ ] Create dashboard binary
- [ ] Generate test dashboard
- [ ] Validate in Grafana

### Phase 5: Testing
- [ ] Run unit tests
- [ ] Run integration tests
- [ ] Manual testing in dev environment
- [ ] Verify metrics in Prometheus

### Phase 6: Cleanup
- [ ] Remove old macro code
- [ ] Update documentation
- [ ] Create migration guide
- [ ] Tag release

## Risk Mitigation

### Backward Compatibility
- Keep existing phase methods unchanged
- Application code doesn't need updates
- Metric names remain the same

### Rollback Plan
1. Git revert to previous commit
2. Restore old dashboards from backup
3. No data loss (Prometheus keeps historical data)

### Testing Strategy
- Unit tests for catalog consistency
- Integration tests for metric recording
- Manual validation of dashboards
- A/B testing with old vs new dashboards

## Timeline

**Total Estimated Time**: 10-14 hours

| Day | Tasks | Hours |
|-----|-------|-------|
| **Day 1** | Phase 1 (Catalog) + Phase 2 (Recorder) | 4-5 |
| **Day 2** | Phase 3 (Runtime Migration) | 3-4 |
| **Day 3** | Phase 4 (Dashboard) + Phase 5 (Testing) | 3-5 |
| **Day 4** | Phase 6 (Cleanup) + Documentation | 1-2 |

## Success Criteria

✅ All 43 metrics defined in central catalog
✅ Type-safe metric keys with compile-time checking
✅ Dashboard automatically generated from catalog
✅ No changes required to application code
✅ All existing functionality preserved
✅ Improved developer experience with IDE autocomplete
✅ Single source of truth for metrics

## Next Steps

1. **Review this plan** with the team
2. **Create feature branch**: `git checkout -b metrics-catalog-migration`
3. **Start with Phase 1**: Create the catalog structure
4. **Incremental commits**: One phase per commit for easy rollback
5. **Test in staging** before production deployment

## Benefits After Migration

- **Add new metric**: Define once in catalog, available everywhere
- **Update dashboard**: Regenerate from catalog
- **Find metrics**: All in one file with documentation
- **Type safety**: Can't reference non-existent metrics
- **Consistency**: Guaranteed alignment between code and dashboards
- **Maintainability**: Reduced code duplication, easier updates
