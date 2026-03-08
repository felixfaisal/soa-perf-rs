/// # Array of Structs vs Struct of Arrays
///
/// This module demonstrates the performance difference between two common
/// data layouts in Rust using a transaction processing example.
///
/// The scenario: you have a large collection of financial transactions and
/// need to perform common operations like summing amounts by status,
/// filtering by time range, or computing running balances. These are
/// everyday operations in databases, block builders, analytics pipelines,
/// and ledger systems.

/// --------------------------------------------------------------------------
/// Array of Structs (AoS)
/// --------------------------------------------------------------------------
/// Each transaction is a single struct. A collection is a Vec of these structs.
///
/// Memory layout of Vec<Transaction>:
/// [amount0 timestamp0 account_id0 status0 category0 metadata0 | amount1 ...]
///  └──────────────────── 120 bytes ───────────────────────────┘
///
/// When your hot loop only needs `amount` and `status` (9 bytes),
/// the CPU still loads the full 120 bytes into cache — including the
/// 64-byte metadata blob you never read.

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TxStatus {
    Pending = 0,
    Completed = 1,
    Failed = 2,
    Cancelled = 3,
}

#[derive(Clone)]
pub struct Transaction {
    pub amount: f64,        // 8 bytes  — hot field
    pub timestamp: u64,     // 8 bytes
    pub account_id: u64,    // 8 bytes
    pub status: TxStatus,   // 1 byte   — hot field
    pub _pad: [u8; 7],      // 7 bytes  — alignment padding
    pub category: u64,      // 8 bytes
    pub metadata: [u8; 80], // 80 bytes — signature, memo, auxiliary data
}
// Total: 120 bytes per transaction — spans 2 cache lines

/// Sum amounts of all completed transactions.
/// This is the most common aggregation in any ledger or analytics system.
/// Only needs `amount` (8 bytes) and `status` (1 byte) per record,
/// but loads 120 bytes per record because the struct is contiguous.
#[inline(never)]
pub fn sum_completed_aos(txns: &[Transaction]) -> f64 {
    let mut sum = 0.0f64;
    for tx in txns {
        if tx.status == TxStatus::Completed {
            sum += tx.amount;
        }
    }
    sum
}

/// Count transactions in a time range.
/// Only needs `timestamp`, loads entire 120-byte struct per access.
#[inline(never)]
pub fn count_in_range_aos(txns: &[Transaction], start: u64, end: u64) -> usize {
    let mut count = 0usize;
    for tx in txns {
        if tx.timestamp >= start && tx.timestamp <= end {
            count += 1;
        }
    }
    count
}

/// Compute total volume (sum of all amounts regardless of status).
/// Only needs `amount`, loads 120 bytes per record.
#[inline(never)]
pub fn total_volume_aos(txns: &[Transaction]) -> f64 {
    let mut sum = 0.0f64;
    for tx in txns {
        sum += tx.amount;
    }
    sum
}

/// --------------------------------------------------------------------------
/// Struct of Arrays (SoA)
/// --------------------------------------------------------------------------
/// Each field is stored in its own contiguous array.
///
/// Memory layout:
/// amounts:    [a0 a1 a2 a3 a4 a5 a6 a7 ...]   — 8 f64s per cache line
/// statuses:   [s0 s1 s2 ... s63 ...]           — 64 statuses per cache line
/// timestamps: [t0 t1 t2 t3 t4 t5 t6 t7 ...]   — untouched during amount ops
/// account_ids:[...]                             — untouched during amount ops
/// categories: [...]                             — untouched during amount ops
/// metadata:   [...]                             — untouched during amount ops
///
/// When summing completed amounts, only `amounts` and `statuses` enter the cache.
/// The `statuses` array is especially cache-friendly: at 1 byte each,
/// 64 statuses fit in a single cache line vs 1 status per 2 cache lines in AoS.

pub struct Transactions {
    pub amounts: Vec<f64>,
    pub timestamps: Vec<u64>,
    pub account_ids: Vec<u64>,
    pub statuses: Vec<TxStatus>,
    pub categories: Vec<u64>,
    pub metadata: Vec<[u8; 80]>,
}

impl Transactions {
    pub fn len(&self) -> usize {
        self.amounts.len()
    }
}

/// Sum amounts of all completed transactions.
/// Only `amounts` and `statuses` arrays are loaded into cache.
/// `statuses` packs 64 values per cache line (1 byte each).
/// `amounts` packs 8 values per cache line (8 bytes each).
/// The prefetcher handles both arrays efficiently due to sequential access.
#[inline(never)]
pub fn sum_completed_soa(txns: &Transactions) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..txns.len() {
        if txns.statuses[i] == TxStatus::Completed {
            sum += txns.amounts[i];
        }
    }
    sum
}

/// Count transactions in a time range.
/// Only `timestamps` array is loaded — 8 values per cache line.
#[inline(never)]
pub fn count_in_range_soa(txns: &Transactions, start: u64, end: u64) -> usize {
    let mut count = 0usize;
    for i in 0..txns.len() {
        if txns.timestamps[i] >= start && txns.timestamps[i] <= end {
            count += 1;
        }
    }
    count
}

/// Compute total volume.
/// Only `amounts` array is loaded — 8 values per cache line, perfect for SIMD.
#[inline(never)]
pub fn total_volume_soa(txns: &Transactions) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..txns.len() {
        sum += txns.amounts[i];
    }
    sum
}

/// --------------------------------------------------------------------------
/// Data generation helpers
/// --------------------------------------------------------------------------

pub fn generate_aos(n: usize) -> Vec<Transaction> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let statuses = [
        TxStatus::Pending,
        TxStatus::Completed,
        TxStatus::Failed,
        TxStatus::Cancelled,
    ];

    (0..n)
        .map(|i| Transaction {
            amount: rng.gen_range(0.01..10_000.0),
            timestamp: 1_700_000_000 + (i as u64) + rng.gen_range(0..1000),
            account_id: rng.gen_range(1..100_000),
            status: statuses[rng.gen_range(0..4)],
            _pad: [0u8; 7],
            category: rng.gen_range(0..50),
            metadata: [0u8; 80],
        })
        .collect()
}

pub fn generate_soa(n: usize) -> Transactions {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let statuses_pool = [
        TxStatus::Pending,
        TxStatus::Completed,
        TxStatus::Failed,
        TxStatus::Cancelled,
    ];

    let mut amounts = Vec::with_capacity(n);
    let mut timestamps = Vec::with_capacity(n);
    let mut account_ids = Vec::with_capacity(n);
    let mut statuses = Vec::with_capacity(n);
    let mut categories = Vec::with_capacity(n);
    let mut metadata = Vec::with_capacity(n);

    for i in 0..n {
        amounts.push(rng.gen_range(0.01..10_000.0));
        timestamps.push(1_700_000_000 + (i as u64) + rng.gen_range(0..1000));
        account_ids.push(rng.gen_range(1..100_000));
        statuses.push(statuses_pool[rng.gen_range(0..4)]);
        categories.push(rng.gen_range(0..50));
        metadata.push([0u8; 80]);
    }

    Transactions {
        amounts,
        timestamps,
        account_ids,
        statuses,
        categories,
        metadata,
    }
}
