/// Standalone binary for profiling AoS layout with `perf`.
///
/// Usage:
///   cargo build --release --bin bench_aos
///   perf stat -e cache-misses,cache-references,L1-dcache-load-misses,LLC-load-misses \
///       ./target/release/bench_aos
use soa_perf_rs::*;

fn main() {
    let n = 1_000_000;
    let txns = generate_aos(n);

    let mut total = 0.0f64;
    for _ in 0..100 {
        total += sum_completed_aos(&txns);
        total += total_volume_aos(&txns);
    }

    println!("AoS total: {total}");
}
