use std::time::Instant;
use scratchpad::json_escape_SWAR::{has_json_escapable_byte, has_json_escapable_byte_scalar};

fn bench_with_timing(name: &str, f: impl Fn() -> bool, iterations: usize, input_size: usize) -> f64 {
    // Warmup
    for _ in 0..10 {
        std::hint::black_box(f());
    }

    let start = Instant::now();
    let mut total_bytes = 0;

    for _ in 0..iterations {
        let result = f();
        total_bytes += input_size;
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

    throughput_gb_s
}

fn main() {
    println!("=== JSON Escape Detection Benchmarks (SWAR) ===\n");

    // Test 1: Clean ASCII (no escapable characters)
    println!("--- Clean ASCII (no escapable chars) ---");
    let clean_input: Vec<u8> = (32..127).cycle().take(1_000_000).collect();
    let iterations = 1_000;

    let scalar_clean = bench_with_timing(
        "Scalar (clean, 1 MB)",
        || has_json_escapable_byte_scalar(&clean_input),
        iterations,
        clean_input.len(),
    );

    let swar_clean = bench_with_timing(
        "SWAR (clean, 1 MB)",
        || has_json_escapable_byte(&clean_input),
        iterations,
        clean_input.len(),
    );

    println!();

    // Test 2: With escapable characters (early exit scenario)
    println!("--- With escapable chars (early detection) ---");
    let mut early_escape = vec![65u8; 1_000_000]; // All 'A'
    early_escape[100] = b'"'; // Add quote at position 100

    let scalar_early = bench_with_timing(
        "Scalar (early escape, 1 MB)",
        || has_json_escapable_byte_scalar(&early_escape),
        iterations,
        early_escape.len(),
    );

    let swar_early = bench_with_timing(
        "SWAR (early escape, 1 MB)",
        || has_json_escapable_byte(&early_escape),
        iterations,
        early_escape.len(),
    );

    println!();

    // Test 3: Mixed content with various escapable chars
    println!("--- Mixed content (quotes, backslashes, newlines) ---");
    let mut mixed_input = Vec::with_capacity(1_000_000);
    for i in 0..1_000_000 {
        let byte = match i % 100 {
            10 => b'"',   // Quote every 100 bytes
            25 => b'\\',  // Backslash
            50 => b'\n',  // Newline
            75 => b'\t',  // Tab
            _ => (65 + (i % 26)) as u8, // Letters A-Z
        };
        mixed_input.push(byte);
    }

    let scalar_mixed = bench_with_timing(
        "Scalar (mixed, 1 MB)",
        || has_json_escapable_byte_scalar(&mixed_input),
        iterations,
        mixed_input.len(),
    );

    let swar_mixed = bench_with_timing(
        "SWAR (mixed, 1 MB)",
        || has_json_escapable_byte(&mixed_input),
        iterations,
        mixed_input.len(),
    );

    println!();

    // Test 4: Different input sizes
    println!("--- Different input sizes (clean ASCII) ---");
    for size_kb in [1, 10, 100, 1000] {
        let size_bytes = size_kb * 1024;
        let input: Vec<u8> = (32..127).cycle().take(size_bytes).collect();
        let iter_count = (1_000_000 / size_bytes).max(10);

        println!("  {} KB:", size_kb);
        let scalar_size = bench_with_timing(
            "    Scalar",
            || has_json_escapable_byte_scalar(&input),
            iter_count,
            input.len(),
        );

        let swar_size = bench_with_timing(
            "    SWAR",
            || has_json_escapable_byte(&input),
            iter_count,
            input.len(),
        );

        println!();
    }

    // Test 5: Very large input (10 MB)
    println!("--- Very large input (10 MB, clean ASCII) ---");
    let very_large_input: Vec<u8> = (32..127).cycle().take(10_000_000).collect();
    let iterations_large = 100;

    let scalar_large = bench_with_timing(
        "Scalar (10 MB)",
        || has_json_escapable_byte_scalar(&very_large_input),
        iterations_large,
        very_large_input.len(),
    );

    let swar_large = bench_with_timing(
        "SWAR (10 MB)",
        || has_json_escapable_byte(&very_large_input),
        iterations_large,
        very_large_input.len(),
    );

    println!();

    // Test 6: Worst case - escapable char at the end
    println!("--- Worst case (escapable at end) ---");
    let mut worst_case = vec![65u8; 1_000_000]; // All 'A'
    worst_case[999_999] = b'"'; // Quote at the very end

    let scalar_worst = bench_with_timing(
        "Scalar (worst case, 1 MB)",
        || has_json_escapable_byte_scalar(&worst_case),
        iterations,
        worst_case.len(),
    );

    let swar_worst = bench_with_timing(
        "SWAR (worst case, 1 MB)",
        || has_json_escapable_byte(&worst_case),
        iterations,
        worst_case.len(),
    );

    println!();
}
