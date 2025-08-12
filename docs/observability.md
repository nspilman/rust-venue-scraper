# Observability: Prometheus, Pushgateway, and Grafana

This document explains how the project’s observability stack is wired together, how metrics flow end-to-end, and how to run, verify, and troubleshoot it locally.

- Prometheus scrapes metrics and stores time series data.
- Pushgateway receives one-shot metrics that the scraper pushes on pipeline completion.
- Grafana visualizes both scraped exporter metrics and the pushed run metrics via provisioned dashboards.

Contents
- Architecture Overview
- Components and Ports
- Metrics produced by the scraper
- How metrics flow (two paths)
- Configuration
  - docker-compose
  - Prometheus
  - Grafana provisioning (datasource + dashboard)
- Running locally
- Verifying data end-to-end
- Common Prometheus queries
- Troubleshooting
- Notes and best practices


## Architecture Overview

There are two complementary metric paths:

1) Exporter scrape (pull-based): The scraper process exposes a Prometheus exporter HTTP endpoint at /metrics on port 9898. Prometheus scrapes it on a cadence and stores the series (job=sms_scraper).

2) Pushgateway (push-based): At the end of each pipeline run, the scraper pushes a small set of “run result” metrics to the Pushgateway. Prometheus scrapes the Pushgateway (job=pushgateway, honor_labels=true) so that Pushgateway-provided labels (notably instance=<api_name>) are preserved in Prometheus.

Grafana reads from Prometheus and displays both types of metrics. The starter dashboard surfaces last-run metrics (from Pushgateway) and run counts (from either path).


## Components and Ports

- Scraper (container name: sms_scraper)
  - Exposes: 8080 (health), 9898 (Prometheus exporter /metrics)
  - Env: SMS_METRICS_PORT=9898, SMS_PUSHGATEWAY_URL=http://pushgateway:9091
  - On successful pipeline completion: pushes run metrics to Pushgateway

- Pushgateway (container name: pushgateway)
  - Exposes: 9091 (HTTP)
  - Receives one-shot metrics (e.g., sms_ingest_runs_total) from the scraper

- Prometheus (container name: prometheus)
  - Exposes: 9090 (UI and HTTP API)
  - Scrapes:
    - Scraper exporter at http://host.docker.internal:9898/metrics
    - Pushgateway at http://pushgateway:9091/metrics (honor_labels=true)
  - Persists data in volume: prometheus_data

- Grafana (container name: grafana)
  - Exposes: 3000 (UI and HTTP API)
  - Provisioned Prometheus datasource (uid=prometheus)
  - Provisioned dashboard: “SMS Scraper Overview” (folder: SMS)


## Metrics produced by the scraper

The scraper emits two sets of metrics:

1) Exporter metrics (via the metrics crate and exporter at :9898):
   - Counters/Histograms emitted during processing, e.g.,
     - sms_events_processed_total
     - sms_events_skipped_total
     - sms_pipeline_errors_total
     - sms_pipeline_duration_seconds_bucket (histogram)
   - These are scraped on a cadence by Prometheus from the exporter.

2) Pushgateway run metrics (one-shot per pipeline run):
   - Pushed after the pipeline completes successfully.
   - Include:
     - sms_ingest_runs_total
     - sms_events_processed_total
     - sms_events_skipped_total
     - sms_pipeline_errors_total
     - sms_pipeline_duration_seconds
   - Labeled with instance="<api_name>" so runs for different APIs are distinguishable (e.g., blue_moon, sea_monster, darrells_tavern).

Note: For histograms during pushgateway reporting we send a single duration value (seconds) per run; the exporter path provides the full histogram buckets for rate/quantile queries.


## How metrics flow (two paths)

- Exporter path:
  scraper —(HTTP /metrics)→ Prometheus

- Pushgateway path:
  scraper —(HTTP push)→ Pushgateway —(HTTP /metrics)→ Prometheus

Grafana queries Prometheus for both.


## Configuration

### docker-compose

See docker-compose.yml. Key points:

- scraper service
  - Ports: 8080, 9898
  - Env:
    - SMS_METRICS_PORT=9898
    - SMS_PUSHGATEWAY_URL=http://pushgateway:9091
  - Healthcheck on 8080

- pushgateway service
  - Ports: 9091

- prometheus service
  - Ports: 9090
  - Volume: ./ops/prometheus.yml mounted to /etc/prometheus/prometheus.yml (read-only)
  - Data volume: prometheus_data at /prometheus
  - Retention flags set (time and size)

- grafana service (profile: observability)
  - Ports: 3000
  - Env: GF_SECURITY_ADMIN_PASSWORD=admin (first login)
  - Volume: ./ops/grafana/provisioning mounted to /etc/grafana/provisioning (read-only)

### Prometheus

ops/prometheus.yml:

- global.scrape_interval: 15s (change as needed in dev)
- scrape_configs:
  - job_name: sms_scraper
    - targets: [host.docker.internal:9898]
  - job_name: pushgateway
    - honor_labels: true
    - targets: [pushgateway:9091]

Optional: ops/prometheus.local.yml uses a 1s interval and can target multiple local exporters.

Why honor_labels=true for Pushgateway?
- The scraper sets instance=<api_name> on pushed metrics. honor_labels preserves that label rather than overwriting with the scraped target’s labels.

### Grafana provisioning

- Datasource: ops/grafana/provisioning/datasources/datasource.yaml
  - Prometheus datasource with uid=prometheus, url=http://prometheus:9090

- Dashboards: ops/grafana/provisioning/dashboards/
  - dashboard.yaml provider loads JSON dashboards from the same folder into the SMS folder
  - sms-scraper-overview.json contains panels:
    - Ingest Runs by API: sum by (instance) (sms_ingest_runs_total)
    - Events Processed (last run): sum by (instance) (sms_events_processed_total)
    - Events Skipped (last run): sum by (instance) (sms_events_skipped_total)
    - Pipeline Errors (last run): sum by (instance) (sms_pipeline_errors_total)
    - Pipeline Duration (last run): sum by (instance) (sms_pipeline_duration_seconds)

Tip: You can add rate/quantile panels that use exporter histograms, e.g., histogram_quantile over sms_pipeline_duration_seconds_bucket.


## Running locally

- Start Prometheus and Pushgateway (and scraper, which is a dependency):
  docker-compose up -d prometheus pushgateway scraper

- Start Grafana (observability profile):
  docker-compose --profile observability up -d grafana

- First Grafana login:
  - URL: http://localhost:3000
  - Username: admin
  - Password: admin (you’ll be prompted to change it)

- Run a one-shot scraper pipeline to generate metrics immediately:
  docker-compose run --rm scraper sh -lc '/usr/local/bin/sms_scraper ingester --bypass-cadence'


## Verifying data end-to-end

Pushgateway (should show pushed metrics):
- http://localhost:9091/metrics
- Look for lines like:
  sms_ingest_runs_total{instance="blue_moon",job="sms_scraper"} 1

Prometheus targets (should be up):
- http://localhost:9090/targets
  - sms_scraper target: http://host.docker.internal:9898/metrics (up)
  - pushgateway target: http://pushgateway:9091/metrics (up)

Prometheus queries (HTTP API):
- Current run counts:
  curl -sG --data-urlencode 'query=sms_ingest_runs_total' http://localhost:9090/api/v1/query
- Last run processed totals per instance:
  curl -sG --data-urlencode 'query=sum by (instance) (sms_events_processed_total)' http://localhost:9090/api/v1/query

Grafana:
- Open the “SMS Scraper Overview” dashboard (Dashboards → Browse → SMS)
- Refresh to see last-run values populate after a pipeline run


## Common Prometheus queries

- Ingest runs by API:
  sum by (instance) (sms_ingest_runs_total)

- Last-run totals by API (from Pushgateway):
  sum by (instance) (sms_events_processed_total)
  sum by (instance) (sms_events_skipped_total)
  sum by (instance) (sms_pipeline_errors_total)
  sum by (instance) (sms_pipeline_duration_seconds)

- Rates/quantiles using exporter histogram (from the scraper exporter):
  histogram_quantile(0.95, sum by (le, instance) (rate(sms_pipeline_duration_seconds_bucket[5m])))

- Error rate:
  sum by (instance) (rate(sms_pipeline_errors_total[5m]))


## Troubleshooting

- Prometheus query returns empty, but Pushgateway shows metrics
  - Ensure Prometheus has a pushgateway scrape_config with honor_labels: true
  - Check Prometheus targets page for pushgateway being up
  - Wait for the next scrape interval (default 15s) or lower it in ops/prometheus.yml

- Grafana panels show “Unauthorized” when using API
  - Log in to Grafana UI (admin/admin) and obtain a session; for API use, create an API token in Grafana and include it in Authorization headers

- Grafana shows no data in last-run panels
  - Ensure a pipeline run finished and pushed to Pushgateway (the scraper logs “Pushed metrics to Pushgateway for api=<name>”)
  - Verify http://localhost:9091/metrics contains sms_* series
  - Verify Prometheus scrape target for pushgateway is up

- Prometheus sms_scraper target is down
  - On macOS, host.docker.internal resolves from inside Docker. Verify the exporter is bound on 0.0.0.0:9898 inside the scraper container (it is by default)
  - Confirm the scraper container is Healthy

- Grafana provisioning error: “Only one datasource per organization can be marked as default”
  - The datasource is provisioned with isDefault: false to avoid conflicts

- Slow feedback during development
  - Reduce global.scrape_interval to 1s in ops/prometheus.yml (or use ops/prometheus.local.yml) and restart Prometheus


## Notes and best practices

- Labels: The instance label is used to represent the API name for run metrics. honor_labels on the pushgateway job preserves this label.
- Exporter vs Pushgateway: Exporter metrics are ideal for long-running rates and histograms; Pushgateway metrics provide immediate last-run snapshots after batch jobs.
- Persistence:
  - Prometheus data persists in the prometheus_data volume
  - Grafana configuration is provisioned from the repo; its application data (users, dashboards if created via UI) is not persisted by default. Consider adding a data volume for Grafana if you need persistence beyond provisioning.
  - Pushgateway metrics are in-memory and will reset if the container restarts. This is acceptable for last-run snapshots in development; for production, consider persistence or idempotent run IDs.
- Security: Default Grafana admin password is set to “admin” for local dev only; change it on first login. Do not expose these services publicly without proper authentication and network controls.


## File locations (for reference)

- Prometheus: ops/prometheus.yml
- Grafana provisioning:
  - Datasource: ops/grafana/provisioning/datasources/datasource.yaml
  - Dashboard provider: ops/grafana/provisioning/dashboards/dashboard.yaml
  - Dashboard JSON: ops/grafana/provisioning/dashboards/sms-scraper-overview.json
- docker-compose: docker-compose.yml


## Handy commands

- Build and run a one-shot scraper pipeline:
  docker-compose build scraper
  docker-compose run --rm scraper sh -lc '/usr/local/bin/sms_scraper ingester --bypass-cadence'

- Restart Prometheus after config changes:
  docker-compose restart prometheus

- Start Grafana:
  docker-compose --profile observability up -d grafana

- Check Pushgateway metrics:
  curl -s http://localhost:9091/metrics | grep -E '^sms_'

- Query Prometheus for last-run processed totals:
  curl -sG --data-urlencode 'query=sum by (instance) (sms_events_processed_total)' http://localhost:9090/api/v1/query | jq '.'

