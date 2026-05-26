# cw-emf-mocker

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

A Rust CLI tool that generates synthetic HTTP request/response latency events
and emits them as [AWS CloudWatch Embedded Metric Format (EMF)][emf] JSON
records — one record per line on stdout.

## Why

Testing and developing CloudWatch dashboards, alarms, and log-based metrics
requires a steady stream of realistic data. Standing up a real service just to
produce that data is heavyweight. `cw-emf-mocker` gives you a statistically
plausible event stream from a single binary, with no AWS infrastructure
required until you're ready to point it at a real endpoint.

## How it works

**Arrival model** — requests arrive as a [Poisson process][poisson]. The time
between events is drawn from an exponential distribution, which is the
mathematically correct inter-arrival distribution for a Poisson process. At
the default rate of 2 requests/minute the mean wait between events is 30
seconds.

**Response time model** — each event's response time is drawn from a normal
distribution (default μ=2 s, σ=0.5 s), clamped to a minimum of 1 ms to avoid
unphysical negative values in the tail.

**Output** — each simulated request produces a single-line EMF JSON record.
Records are built using the [`metrics_cloudwatch_embedded`][emf-crate] crate,
which formats them correctly for ingestion by CloudWatch Logs. The `Service`
dimension and metric namespace are configurable; every record also carries
`RequestId` and `TaskId` as UUID v7 reference properties (time-sortable,
not metric dimensions).

## Output format

```json
{
  "_aws": {
    "Timestamp": 1748922000000,
    "CloudWatchMetrics": [{
      "Namespace": "RequestSimulator",
      "Dimensions": [["Service"]],
      "Metrics": [
        { "Name": "ResponseTime", "Unit": "Milliseconds" },
        { "Name": "RequestCount", "Unit": "Count" }
      ]
    }]
  },
  "Service": "api",
  "RequestId": "019e647c-b1ca-7000-0000-000000000001",
  "TaskId":    "019e647c-b1ca-7000-0000-000000000002",
  "ResponseTime": [1923.4],
  "RequestCount": 1
}
```

Diagnostic startup information is written to **stderr** so stdout remains
clean for piping.

## Build

Requires Rust (stable). Install via [rustup](https://rustup.rs).

```bash
cargo build --release
```

## Usage

```bash
# Realistic pacing — one event every ~30 seconds on average
cargo run --release -- --print

# Fast mode — no sleep, events as fast as the CPU can produce them
cargo run --release -- --print --fast

# Emit exactly 10 events then exit
cargo run --release -- --print --fast --count 10

# Custom distribution parameters
cargo run --release -- --print \
  --arrival-rate 10 \
  --mean 0.5 \
  --stddev 0.1 \
  --namespace MyApp \
  --service checkout
```

## CLI reference

| Flag | Default | Description |
|---|---|---|
| `--arrival-rate <f64>` | `2.0` | Poisson λ — average requests per minute |
| `--mean <f64>` | `2.0` | Mean response time in seconds |
| `--stddev <f64>` | `0.5` | Response time standard deviation in seconds |
| `--namespace <str>` | `RequestSimulator` | CloudWatch metric namespace |
| `--service <str>` | `api` | Value for the `Service` dimension |
| `--count <u64>` | *(infinite)* | Stop after N events |
| `--print` | *(off)* | Write each EMF record to stdout |
| `--fast` | *(off)* | Skip inter-arrival sleep — useful for testing |

> **Note:** without `--print` records are flushed internally but discarded.
> This default is intentional — it leaves stdout free for a future
> `--post-url` flag that will POST records directly to a CloudWatch Logs
> ingestion endpoint.

## Tests

```bash
cargo test
```

Tests cover CLI argument parsing (defaults and custom values) and verify that
both distributions always produce positive values over 1000 samples. Tests
complete immediately — no sleeping.

## Roadmap

- `--post-url` — POST each flushed record to a CloudWatch Logs / EMF
  ingestion endpoint via an HTTP client.

## License

MIT — see [LICENSE](LICENSE) for details.

[emf]: https://docs.aws.amazon.com/AmazonCloudWatch/latest/monitoring/CloudWatch_Embedded_Metric_Format_Specification.html
[poisson]: https://en.wikipedia.org/wiki/Poisson_point_process
[emf-crate]: https://crates.io/crates/metrics_cloudwatch_embedded
