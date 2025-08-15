# Dashboard Automation

This project includes automated Grafana dashboard generation that dynamically discovers metrics from the codebase and creates comprehensive monitoring dashboards.

## Quick Start

```bash
# Generate and provision dashboard in one command
make dashboard-provision

# Update dashboard after adding new metrics
make dashboard-update
```

## Available Commands

| Command | Description |
|---------|-------------|
| `make dashboard` | Generate static dashboard JSON |
| `make dashboard-dynamic` | Generate dynamic dashboard from MetricName enum |
| `make dashboard-provision` | Generate + copy to Grafana provisioning directory |
| `make dashboard-update` | Update dashboard when new metrics are added |

## How It Works

### Dynamic Generation
The system uses **dynamic generation** that automatically discovers metrics from the `MetricName` enum in your Rust code:

1. **Parses** `src/common/metrics.rs` to extract all metric definitions
2. **Groups** metrics by component (GATEWAY, PARSER, SOURCES, etc.)  
3. **Generates** appropriate Grafana panels based on metric types:
   - Counters → Rate graphs with `rate(metric[5m])`
   - Histograms → Percentile graphs (p50, p95, p99) 
   - Gauges → Current value displays

### Benefits
- ✅ **Type-safe**: Uses the same enum that defines metrics in code
- ✅ **Automatic**: No manual dashboard maintenance required
- ✅ **Consistent**: Standardized panel layouts and queries
- ✅ **Complete**: Discovers all metrics automatically

## Adding New Metrics

When you add new metrics to the `MetricName` enum:

1. **Add** your new metrics to the enum in `src/common/metrics.rs`
2. **Run** `make dashboard-update` to regenerate the dashboard
3. **Restart** Grafana or wait 30 seconds for auto-reload

That's it! Your new metrics will automatically appear in the dashboard with appropriate visualizations.

## File Structure

```
├── src/bin/build-dashboard.rs    # Dashboard generation binary
├── scripts/update-dashboard.sh   # Convenience update script  
├── grafana-dashboard-dynamic.json # Generated dashboard JSON
└── ops/grafana/provisioning/dashboards/
    └── grafana-dashboard-dynamic.json # Provisioned dashboard
```

## Example Workflow

```bash
# 1. Developer adds new metrics to MetricName enum
vim src/common/metrics.rs

# 2. Update dashboard to include new metrics  
make dashboard-update

# 3. Dashboard is automatically updated in Grafana
# Access at http://localhost:3000
```

## Customization

The dashboard generation logic is in `src/bin/build-dashboard.rs`. You can customize:

- Panel layouts and sizing
- Query expressions and time ranges
- Chart types and styling
- Grouping and organization

The system is designed to be easily extensible for your specific monitoring needs.
