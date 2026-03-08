# AoS vs SoA: Data Layout Benchmarks

AoS - Array of Structures 
SoA - Structure of Arrays

Benchmarks demonstrating how data layout affects cache performance in Rust,
using a transaction processing example relevant to databases, block builders,
and ledger systems.

## The Problem

You have 1 million financial transactions. Each transaction record is 120 bytes:

```
struct Transaction {
    amount: f64,         //  8 bytes  ← hot field
    timestamp: u64,      //  8 bytes
    account_id: u64,     //  8 bytes
    status: TxStatus,    //  1 byte   ← hot field
    _pad: [u8; 7],       //  7 bytes
    category: u64,       //  8 bytes
    metadata: [u8; 80],  // 80 bytes  ← cold, never read in hot path
}
```

Your most common query: **"sum all completed transaction amounts"**
This only needs `amount` + `status` = 9 bytes per record.

**AoS layout**: loads 120 bytes per record → 93% of cache space wasted
**SoA layout**: loads only the `amounts` and `statuses` arrays → ~0% wasted

## Running the Benchmarks

### Step 1: Criterion timing benchmarks

```bash
cargo bench
```

Produces HTML reports in `target/criterion/` comparing AoS vs SoA
across three operations at 1K, 10K, 100K, and 1M transactions.

### Step 2: Measure cache misses with perf

```bash
cargo build --release --bin bench_aos --bin bench_soa

perf stat -e cache-misses,cache-references,L1-dcache-load-misses,LLC-load-misses \
    ./target/release/bench_aos

perf stat -e cache-misses,cache-references,L1-dcache-load-misses,LLC-load-misses \
    ./target/release/bench_soa
```

### Step 3: Inspect generated assembly

```bash
cargo install cargo-show-asm

# Compare vectorization between layouts
cargo asm --lib aos_vs_soa::total_volume_aos
cargo asm --lib aos_vs_soa::total_volume_soa
```

The SoA version of `total_volume` should show SIMD instructions
processing multiple f64 values per cycle. The AoS version will likely
use scalar instructions with a 120-byte stride, preventing vectorization.

### Step 4: Line-by-line cache analysis (optional)

```bash
valgrind --tool=cachegrind ./target/release/bench_aos
valgrind --tool=cachegrind ./target/release/bench_soa
cg_annotate cachegrind.out.<pid>
```

## Why This Matters for TEEs

These benchmarks run on normal hardware. Inside an SGX enclave or TDX
confidential VM, every L3 cache miss that reaches DRAM also triggers:

- MEE decryption (AES operations per cache line)
- Integrity verification (Merkle tree traversal)
- Counter write amplification (updates at L1, L2, L3 levels)

The performance gap measured here gets amplified inside TEEs. An optimization
that saves you 2x on normal hardware might save you 3-4x inside an enclave,
because each prevented cache miss avoids both the DRAM latency and the
cryptographic overhead.
