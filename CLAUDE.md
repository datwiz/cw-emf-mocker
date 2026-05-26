# cw-emf-mocker

Rust binary that simulates HTTP request/response latency events and emits them as AWS CloudWatch Embedded Metric Format (EMF) JSON records. Intended to be piped into a log ingestion API.

## Build & run

```bash
cargo build --release
cargo run --release          # stdout → EMF JSON, stderr → diagnostic info
cargo test                   # unit tests only, no sleeping
```

## Architecture

Single file: `src/main.rs`.

**Simulation model**
- Arrivals follow a Poisson process (λ = 2 req/min). Inter-arrival times are sampled from `Exp(λ/60)` (seconds) and the process sleeps between events.
- Response times are sampled from `Normal(μ=2 s, σ=0.5 s)`, clamped to ≥ 1 ms.

**Output**
- One EMF JSON record per line on stdout after each simulated request completes.
- Startup message on stderr so stdout stays pipe-clean.

**EMF record shape**
```json
{
  "_aws": {
    "Timestamp": <unix-ms>,
    "CloudWatchMetrics": [{
      "Namespace": "RequestSimulator",
      "Dimensions": [["Service"]],
      "Metrics": [
        {"Name": "ResponseTime", "Unit": "Milliseconds"},
        {"Name": "RequestCount", "Unit": "Count"}
      ]
    }]
  },
  "Service": "api",
  "RequestId": <u64>,
  "ResponseTime": <f64, 1 decimal place>,
  "RequestCount": 1
}
```

## CLI flags

| Flag | Default | Meaning |
|---|---|---|
| `--arrival-rate` | `2.0` | Poisson λ (requests/minute) |
| `--mean` | `2.0` | Normal μ for response time (seconds) |
| `--stddev` | `0.5` | Normal σ for response time (seconds) |
| `--namespace` | `"RequestSimulator"` | CloudWatch metric namespace |
| `--service` | `"api"` | Value for the `Service` dimension |
| `--count` | *(infinite)* | Stop after N events |
| `--print` | *(off)* | Print each EMF record to stdout |

## Dependencies

| Crate | Purpose |
|---|---|
| `clap 4` (derive) | CLI argument parsing |
| `metrics 0.23` | Facade macros: `histogram!`, `counter!`, `describe_*!` |
| `metrics_cloudwatch_embedded 0.10` | EMF recorder backend; `Builder`, `flush` |
| `rand 0.8` | `thread_rng()` |
| `rand_distr 0.4` | `Exp`, `Normal` distributions |

**Note:** `cloudwatch_namespace` and `with_dimension` require owned `String` (not `&str`) because `SharedString` is `Cow<'static, str>`.

## Planned work

- POST each flushed record to a CloudWatch Logs / EMF ingestion API endpoint via an HTTP client (`ureq` or `reqwest`).
