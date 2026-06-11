# Web Dashboard — SLOs and Alerting

The dashboard (`voxi-web-dashboard`) now exposes golden-signal metrics in
Prometheus text format at `GET /metrics` (unauthenticated, by convention).
This document defines the service level objectives that surface should be
measured against and the burn-rate alerts that page on them.

## Exposed metrics

| Metric | Type | Meaning |
|--------|------|---------|
| `voxi_dashboard_http_requests_total` | counter | All HTTP requests handled (traffic) |
| `voxi_dashboard_http_request_errors_total` | counter | Responses with status >= 500 (errors) |
| `voxi_dashboard_http_in_flight` | gauge | Concurrent in-flight requests (saturation) |
| `voxi_dashboard_http_request_duration_ms` | histogram | Request latency, fixed ms buckets |
| `voxi_dashboard_daemon_up` | gauge | Daemon IPC reachability (1=up, 0=down) |
| `voxi_dashboard_uptime_seconds` | gauge | Dashboard process uptime |

## SLOs

Targets are set against user-visible expectations for a local/edge admin
surface, not a public high-traffic service.

| SLI | Definition | Target (30-day window) |
|-----|------------|------------------------|
| Availability | `1 - (errors / requests)` | 99.5% |
| Latency | fraction of requests < 500 ms | 95% |
| Daemon link | fraction of scrapes with `daemon_up == 1` | 99% |

Error budget (availability): `(1 - 0.995) * total_requests` over the window.
At 99.5%, that is 0.5% of requests, e.g. 5,000 of 1,000,000.

## PromQL — golden signals

```promql
# Traffic
sum(rate(voxi_dashboard_http_requests_total[5m]))

# Error ratio
sum(rate(voxi_dashboard_http_request_errors_total[5m]))
  /
sum(rate(voxi_dashboard_http_requests_total[5m]))

# Latency — p95 (ms)
histogram_quantile(0.95,
  sum(rate(voxi_dashboard_http_request_duration_ms_bucket[5m])) by (le))

# Saturation
max_over_time(voxi_dashboard_http_in_flight[5m])
```

## Alerting — multiwindow burn rate

```yaml
groups:
  - name: voxi_dashboard_slo
    rules:
      # Fast burn: 2% of the monthly error budget in 1h (14.4x).
      - alert: DashboardHighErrorBurn
        expr: |
          (
            sum(rate(voxi_dashboard_http_request_errors_total[1h]))
            / sum(rate(voxi_dashboard_http_requests_total[1h]))
          ) > 0.072
          and
          (
            sum(rate(voxi_dashboard_http_request_errors_total[5m]))
            / sum(rate(voxi_dashboard_http_requests_total[5m]))
          ) > 0.072
        for: 2m
        labels: { severity: critical }
        annotations:
          summary: "Dashboard burning error budget fast"

      # Slow burn: sustained budget consumption over 6h.
      - alert: DashboardSlowErrorBurn
        expr: |
          (
            sum(rate(voxi_dashboard_http_request_errors_total[6h]))
            / sum(rate(voxi_dashboard_http_requests_total[6h]))
          ) > 0.005
        for: 15m
        labels: { severity: warning }
        annotations:
          summary: "Dashboard sustained error budget consumption"

      # Daemon link down: the dashboard is up but cannot reach the agent.
      - alert: DashboardDaemonLinkDown
        expr: voxi_dashboard_daemon_up == 0
        for: 5m
        labels: { severity: critical }
        annotations:
          summary: "Dashboard cannot reach the agent daemon over IPC"
```

## Notes

- The 14.4x fast-burn threshold (0.072) is derived from a 99.5% SLO: at that
  rate the entire 30-day budget burns in ~2 days, warranting a page.
- `daemon_up` is the most actionable signal: the dashboard process can be
  healthy while the agent IPC link is broken, which users experience as a dead
  UI. Alert on it directly.
