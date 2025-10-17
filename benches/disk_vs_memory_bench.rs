use std::time::Instant;
use std::fs::{self, File};
use std::io::Write;
use scratchpad::csv_parse::{count_pattern_matches_from_file, count_pattern_matches_in_memory};

fn write_test_file(file_path: &str, num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "Name,University,Year,GPA,Major")?;
    for i in 0..num_rows {
        writeln!(
            file,
            "Person{},Harvard,{},{:.2},ComputerScience",
            i, 2020 + (i % 5), 3.0 + ((i % 10) as f64 / 10.0)
        )?;
    }
    Ok(())
}

fn bench(name: &str, f: impl Fn() -> usize, iterations: usize, file_size: u64) -> (f64, f64) {
    // Warmup
    for _ in 0..10 {
        std::hint::black_box(f());
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let result = f();
        std::hint::black_box(result);
    }
    let elapsed = start.elapsed().as_secs_f64();

    let total_bytes = file_size as f64 * iterations as f64;
    let throughput = total_bytes / elapsed / 1_000_000_000.0;
    let time_per_op = (elapsed / iterations as f64) * 1000.0; // ms

    println!("{:30} {:>10.2} ms/op, {:>8.2} GB/s", name, time_per_op, throughput);

    (throughput, time_per_op)
}

fn main() {
    println!("=== Disk Buffering vs In-Memory Comparison ===\n");

    // Test different file sizes
    let test_cases = vec![
        ("Small (1K rows, ~50 KB)", 1_000, 1000),
        ("Medium (10K rows, ~500 KB)", 10_000, 200),
        ("Large (100K rows, ~5 MB)", 100_000, 100),
        ("XLarge (200K rows, ~10 MB)", 200_000, 50),
    ];

    for (desc, num_rows, iterations) in test_cases {
        let test_file = format!("/tmp/test_{}.csv", num_rows);

        println!("--- {} ---", desc);
        write_test_file(&test_file, num_rows).unwrap();
        let file_size = fs::metadata(&test_file).unwrap().len();
        println!("  File size: {:.2} MB", file_size as f64 / 1_000_000.0);

        let (throughput_disk, time_disk) = bench(
            "Disk (4KB buffered)",
            || count_pattern_matches_from_file(&test_file, b"Harvard").unwrap(),
            iterations,
            file_size,
        );

        let (throughput_mem, time_mem) = bench(
            "In-Memory (load all)",
            || count_pattern_matches_in_memory(&test_file, b"Harvard").unwrap(),
            iterations,
            file_size,
        );

        let speedup = throughput_mem / throughput_disk;
        let time_diff = ((time_mem - time_disk) / time_disk) * 100.0;

        println!("  → In-Memory is {:.2}x faster ({:+.1}% time)", speedup, -time_diff);
        println!();

        let _ = fs::remove_file(test_file);
    }

    println!("\n=== Analysis ===");
    println!("\nDisk Buffering (4KB):");
    println!("  ✓ Low memory footprint (4 KB buffer)");
    println!("  ✓ Can handle files larger than RAM");
    println!("  ✓ Streaming - starts processing immediately");
    println!("  ✗ Syscall overhead (read() per 4KB chunk)");
    println!("  ✗ Buffer boundary handling complexity");

    println!("\nIn-Memory (load all):");
    println!("  ✓ Single read() syscall - minimal overhead");
    println!("  ✓ Simpler code - no buffer management");
    println!("  ✓ Better cache locality (contiguous memory)");
    println!("  ✓ OS may have already cached the file");
    println!("  ✗ Memory usage = file size");
    println!("  ✗ Cannot handle files larger than RAM");
    println!("  ✗ Must wait for entire file to load");

    println!("\nRecommendation:");
    println!("  For files < 100 MB:  Use in-memory (simpler & faster)");
    println!("  For files > 100 MB:  Use disk buffering (memory safe)");
    println!("  For production:      Use in-memory with size check fallback");
}
