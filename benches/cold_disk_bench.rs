use std::time::Instant;
use std::fs::{self, File};
use std::io::Write;
use std::process::Command;
use scratchpad::csv_parse_buffer_size_impact::{count_pattern_matches_from_file, count_pattern_matches_in_memory};

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

fn clear_os_cache() {
    // On macOS, purge clears the disk cache
    // Note: This requires sudo, so it may not work without privileges
    let _ = Command::new("purge").output();

    // Small delay to ensure cache is cleared
    std::thread::sleep(std::time::Duration::from_millis(100));
}

fn bench_cold(name: &str, f: impl Fn() -> usize, iterations: usize, file_size: u64, clear_cache: bool) -> (f64, f64, f64) {
    let mut times = Vec::new();

    for i in 0..iterations {
        if clear_cache && i > 0 {
            // Clear cache between iterations (except first for warmup)
            clear_os_cache();
        }

        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed().as_secs_f64();

        std::hint::black_box(result);
        times.push(elapsed);
    }

    // First iteration is warmup, use remaining for stats
    let measurement_times = &times[1..];

    let total_time: f64 = measurement_times.iter().sum();
    let avg_time = total_time / measurement_times.len() as f64;
    let min_time = measurement_times.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_time = measurement_times.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let throughput = (file_size as f64) / avg_time / 1_000_000_000.0;

    println!("{:30} avg: {:>7.2} ms, min: {:>7.2} ms, max: {:>7.2} ms | {:>8.2} GB/s",
             name, avg_time * 1000.0, min_time * 1000.0, max_time * 1000.0, throughput);

    (throughput, avg_time, max_time)
}

fn main() {
    println!("=== Cold Disk vs Hot Cache Comparison ===\n");
    println!("Testing with OS page cache cleared between runs\n");

    // Check if we can clear cache
    let can_purge = Command::new("which").arg("purge").output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !can_purge {
        println!("⚠️  Warning: 'purge' command not available.");
        println!("   Results will show cached performance only.\n");
    } else {
        println!("✓ Using 'purge' to clear OS cache between iterations\n");
    }

    let test_cases = vec![
        ("Small (1K rows, ~50 KB)", 1_000, "/tmp/test_cold_1k.csv"),
        ("Medium (10K rows, ~500 KB)", 10_000, "/tmp/test_cold_10k.csv"),
        ("Large (100K rows, ~5 MB)", 100_000, "/tmp/test_cold_100k.csv"),
        ("XLarge (200K rows, ~10 MB)", 200_000, "/tmp/test_cold_200k.csv"),
    ];

    for (desc, num_rows, test_file) in test_cases {
        println!("=== {} ===", desc);
        write_test_file(test_file, num_rows).unwrap();
        let file_size = fs::metadata(test_file).unwrap().len();
        println!("File size: {:.2} MB\n", file_size as f64 / 1_000_000.0);

        let iterations = if can_purge { 11 } else { 101 }; // 1 warmup + 10 or 100 measurements

        // Test with COLD cache (purge between iterations)
        if can_purge {
            println!("--- COLD DISK (cache cleared) ---");

            let (tp_disk_cold, _, max_disk_cold) = bench_cold(
                "Disk (4KB buffered)",
                || count_pattern_matches_from_file(test_file, b"Harvard").unwrap(),
                iterations,
                file_size,
                true, // Clear cache
            );

            let (tp_mem_cold, _, max_mem_cold) = bench_cold(
                "In-Memory (load all)",
                || count_pattern_matches_in_memory(test_file, b"Harvard").unwrap(),
                iterations,
                file_size,
                true, // Clear cache
            );

            let speedup_cold = tp_mem_cold / tp_disk_cold;
            println!("  → Cold: In-Memory is {:.2}x {}",
                     speedup_cold,
                     if speedup_cold >= 1.0 { "faster" } else { "slower" });
            println!();
        }

        // Test with HOT cache (no purge - cached in memory)
        println!("--- HOT CACHE (already in RAM) ---");

        let (tp_disk_hot, _, _) = bench_cold(
            "Disk (4KB buffered)",
            || count_pattern_matches_from_file(test_file, b"Harvard").unwrap(),
            iterations,
            file_size,
            false, // Don't clear cache
        );

        let (tp_mem_hot, _, _) = bench_cold(
            "In-Memory (load all)",
            || count_pattern_matches_in_memory(test_file, b"Harvard").unwrap(),
            iterations,
            file_size,
            false, // Don't clear cache
        );

        let speedup_hot = tp_mem_hot / tp_disk_hot;
        println!("  → Hot: In-Memory is {:.2}x faster", speedup_hot);
        println!();

        let _ = fs::remove_file(test_file);
    }

    println!("\n=== Analysis ===");
    if can_purge {
        println!("\nCOLD DISK (cache cleared):");
        println!("  - Both methods must read from actual disk");
        println!("  - Disk I/O time dominates (~5-20ms for mechanical, ~0.1-1ms for SSD)");
        println!("  - Processing time becomes negligible compared to I/O");
        println!("  - Buffered approach may be more consistent (streaming)");

        println!("\nHOT CACHE (already in RAM):");
        println!("  - File is already in OS page cache");
        println!("  - No actual disk I/O occurs");
        println!("  - In-memory is faster due to single syscall vs many");
        println!("  - This was our previous benchmark scenario");
    } else {
        println!("\nHOT CACHE only (cache not cleared):");
        println!("  - Results show best-case scenario");
        println!("  - Real-world: first access is cold, subsequent are hot");
        println!("  - Run with sudo to enable cache clearing");
    }

    println!("\nKey Insight:");
    println!("  When disk I/O dominates (cold cache), buffering strategy matters less.");
    println!("  When data is hot (in cache), syscall overhead matters more.");
}
