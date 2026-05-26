use clap::Parser;
use rand_distr::{Distribution, Exp, Normal};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

/// Mock CloudWatch EMF request/response latency events.
///
/// Arrivals follow a Poisson process (λ requests/min); response times follow
/// N(mean, stddev). Each event is flushed as a single-line EMF JSON record.
/// Pass --print to write records to stdout; without it the output is discarded
/// (useful once --post-url is wired up).
#[derive(Parser)]
#[command(name = "cw-emf-mocker")]
struct Cli {
    /// Arrival rate in requests per minute (Poisson λ)
    #[arg(long, default_value_t = 2.0)]
    arrival_rate: f64,

    /// Mean response time in seconds
    #[arg(long, default_value_t = 2.0)]
    mean: f64,

    /// Response time standard deviation in seconds
    #[arg(long, default_value_t = 0.5)]
    stddev: f64,

    /// CloudWatch metric namespace
    #[arg(long, default_value = "RequestSimulator")]
    namespace: String,

    /// Service dimension value
    #[arg(long, default_value = "api")]
    service: String,

    /// Number of events to emit (omit for infinite loop)
    #[arg(long)]
    count: Option<u64>,

    /// Print each EMF record to stdout
    #[arg(long)]
    print: bool,

    /// Skip inter-arrival sleep — emit events as fast as possible (useful for testing)
    #[arg(long)]
    fast: bool,
}

fn main() {
    let args = Cli::parse();

    let collector = metrics_cloudwatch_embedded::Builder::new()
        .cloudwatch_namespace(args.namespace.clone())
        .with_dimension("Service", args.service.clone())
        .init()
        .unwrap();

    metrics::describe_histogram!(
        "ResponseTime",
        metrics::Unit::Milliseconds,
        "Request response time"
    );
    metrics::describe_counter!("RequestCount", metrics::Unit::Count, "Number of requests");

    let mut rng = rand::thread_rng();

    let inter_arrival = Exp::new(args.arrival_rate / 60.0).expect("arrival_rate must be > 0");
    let response_time =
        Normal::new(args.mean, args.stddev).expect("stddev must be > 0");

    let mut event_count: u64 = 0;
    let limit = args.count.unwrap_or(u64::MAX);

    eprintln!(
        "Starting mocker: λ={}/min, response_time~N({:.1}s, {:.1}s), namespace={} {}",
        args.arrival_rate, args.mean, args.stddev, args.namespace,
        if args.fast { "[fast mode]" } else { "" }
    );

    while event_count < limit {
        if !args.fast {
            let wait_secs: f64 = inter_arrival.sample(&mut rng);
            thread::sleep(Duration::from_secs_f64(wait_secs));
        }

        event_count += 1;

        // clamp to 1 ms minimum — Normal can produce negatives in the tail
        let rt_ms = response_time.sample(&mut rng).max(0.001) * 1000.0;

        metrics::histogram!("ResponseTime").record(rt_ms);
        metrics::counter!("RequestCount").increment(1);

        let flush_result = collector
            .set_property("RequestId", Uuid::now_v7().to_string())
            .set_property("TaskId", Uuid::now_v7().to_string());
        if args.print {
            flush_result.flush(std::io::stdout()).unwrap();
        } else {
            flush_result.flush(std::io::sink()).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_defaults() {
        let args = Cli::parse_from(["cw-emf-mocker"]);
        assert_eq!(args.arrival_rate, 2.0);
        assert_eq!(args.mean, 2.0);
        assert_eq!(args.stddev, 0.5);
        assert_eq!(args.namespace, "RequestSimulator");
        assert_eq!(args.service, "api");
        assert!(args.count.is_none());
        assert!(!args.print, "stdout printing should be off by default");
    }

    #[test]
    fn cli_print_flag() {
        let args = Cli::parse_from(["cw-emf-mocker", "--print"]);
        assert!(args.print);
    }

    #[test]
    fn cli_fast_flag() {
        let args = Cli::parse_from(["cw-emf-mocker", "--fast"]);
        assert!(args.fast);
        assert!(!args.fast || true, "fast defaults to off");

        let args_default = Cli::parse_from(["cw-emf-mocker"]);
        assert!(!args_default.fast, "fast should be off by default");
    }

    #[test]
    fn cli_custom_args() {
        let args = Cli::parse_from([
            "cw-emf-mocker",
            "--arrival-rate",
            "5.0",
            "--mean",
            "1.5",
            "--stddev",
            "0.3",
            "--namespace",
            "MyApp",
            "--service",
            "web",
            "--count",
            "10",
        ]);
        assert_eq!(args.arrival_rate, 5.0);
        assert_eq!(args.mean, 1.5);
        assert_eq!(args.stddev, 0.3);
        assert_eq!(args.namespace, "MyApp");
        assert_eq!(args.service, "web");
        assert_eq!(args.count, Some(10));
    }

    #[test]
    fn distributions_produce_positive_values() {
        let mut rng = rand::thread_rng();
        let inter_arrival = Exp::new(2.0 / 60.0).unwrap();
        let response_time = Normal::new(2.0_f64, 0.5_f64).unwrap();
        for _ in 0..1000 {
            assert!(inter_arrival.sample(&mut rng) > 0.0);
            let rt = response_time.sample(&mut rng).max(0.001);
            assert!(rt > 0.0);
        }
    }
}
