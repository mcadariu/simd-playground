use std::time::Instant;
use scratchpad::line_feed_every_k_bytes::{insert_line_feed_neon, insert_line_feed_scalar};

fn bench_with_timing(name: &str, f: impl Fn() -> Vec<u8>, iterations: usize) -> (f64, usize) {
    // Warmup
    for _ in 0..10 {
        std::hint::black_box(f());
    }

    let start = Instant::now();
    let mut total_bytes = 0;

    for _ in 0..iterations {
        let result = f();
        total_bytes += result.len();
        std::hint::black_box(result);
    }

    let elapsed = start.elapsed();
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput_gb_s = (total_bytes as f64 / elapsed_secs) / 1_000_000_000.0;

    println!(
        "{:30} {:.2} ms total, {:.2} GB/s throughput",
        format!("{}:", name),
        elapsed_secs * 1000.0,
        throughput_gb_s
    );

    (throughput_gb_s, total_bytes)
}

fn main() {
    println!("=== Line Feed Insertion Benchmarks (ARM NEON) ===\n");

    // Large input: 1 MB
    println!("--- Large input (1 MB, K=64) ---");
    let large_input: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();
    let iterations_large = 1_000;

    bench_with_timing(
        "Scalar (large)",
        || insert_line_feed_scalar(&large_input, 64),
        iterations_large,
    );

    bench_with_timing(
        "NEON (large)",
        || insert_line_feed_neon(&large_input, 64),
        iterations_large,
    );
    println!();

    // Very large input: 10 MB
    println!("--- Very large input (10 MB, K=64) ---");
    let very_large_input: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();
    let iterations_very_large = 100;

    bench_with_timing(
        "Scalar (very large)",
        || insert_line_feed_scalar(&very_large_input, 64),
        iterations_very_large,
    );

    bench_with_timing(
        "NEON (very large)",
        || insert_line_feed_neon(&very_large_input, 64),
        iterations_very_large,
    );
    println!();

    // Test different K values with 1 MB input
    println!("--- Different K values (1 MB input) ---");
    let test_input: Vec<u8> = (0..1_000_000).map(|i| (i % 256) as u8).collect();

    for k in [32, 64, 72, 128] {
        bench_with_timing(
            &format!("Scalar (K={})", k),
            || insert_line_feed_scalar(&test_input, k),
            500,
        );
        bench_with_timing(
            &format!("NEON (K={})", k),
            || insert_line_feed_neon(&test_input, k),
            500,
        );
        println!();
    }
}
