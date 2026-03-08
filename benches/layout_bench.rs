use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use soa_perf_rs::*;

/// --------------------------------------------------------------------------
/// Benchmark: Sum completed transaction amounts
/// The most common query — "what's my total revenue?"
/// Only needs `amount` and `status` fields.
/// --------------------------------------------------------------------------
fn bench_sum_completed(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum_completed");

    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let aos_data = generate_aos(n);
        let soa_data = generate_soa(n);

        group.bench_with_input(BenchmarkId::new("AoS", n), &n, |b, _| {
            b.iter(|| black_box(sum_completed_aos(black_box(&aos_data))))
        });

        group.bench_with_input(BenchmarkId::new("SoA", n), &n, |b, _| {
            b.iter(|| black_box(sum_completed_soa(black_box(&soa_data))))
        });
    }
    group.finish();
}

/// --------------------------------------------------------------------------
/// Benchmark: Count transactions in a time range
/// Common for analytics — "how many transactions in the last hour?"
/// Only needs `timestamp` field.
/// --------------------------------------------------------------------------
fn bench_count_in_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("count_in_range");

    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let aos_data = generate_aos(n);
        let soa_data = generate_soa(n);

        // Pick a range that covers roughly 25% of the data
        let start = 1_700_000_000u64 + (n as u64 / 4);
        let end = start + (n as u64 / 2);

        group.bench_with_input(BenchmarkId::new("AoS", n), &n, |b, _| {
            b.iter(|| black_box(count_in_range_aos(black_box(&aos_data), start, end)))
        });

        group.bench_with_input(BenchmarkId::new("SoA", n), &n, |b, _| {
            b.iter(|| black_box(count_in_range_soa(black_box(&soa_data), start, end)))
        });
    }
    group.finish();
}

/// --------------------------------------------------------------------------
/// Benchmark: Total volume (sum all amounts)
/// Simplest aggregation — only touches one field.
/// This is where SoA should show the biggest advantage because
/// the compiler can fully vectorize a tight loop over contiguous f64s.
/// --------------------------------------------------------------------------
fn bench_total_volume(c: &mut Criterion) {
    let mut group = c.benchmark_group("total_volume");

    for &n in &[1_000, 10_000, 100_000, 1_000_000] {
        let aos_data = generate_aos(n);
        let soa_data = generate_soa(n);

        group.bench_with_input(BenchmarkId::new("AoS", n), &n, |b, _| {
            b.iter(|| black_box(total_volume_aos(black_box(&aos_data))))
        });

        group.bench_with_input(BenchmarkId::new("SoA", n), &n, |b, _| {
            b.iter(|| black_box(total_volume_soa(black_box(&soa_data))))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_sum_completed,
    bench_count_in_range,
    bench_total_volume
);
criterion_main!(benches);
