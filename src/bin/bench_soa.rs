/// Standalone binary for profiling SoA layout with `perf`.
///
/// Usage:
///   cargo build --release --bin bench_soa
///   perf stat -e cache-misses,cache-references,L1-dcache-load-misses,LLC-load-misses \
///       ./target/release/bench_soa
use soa_perf_rs::*;

fn main() {
    let n = 1_000_000;
    let txns = generate_soa(n);

    let mut total = 0.0f64;
    for _ in 0..100 {
        total += sum_completed_soa(&txns);
        total += total_volume_soa(&txns);
    }

    println!("SoA total: {total}");
}
