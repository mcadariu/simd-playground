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

fn bench(name: &str, f: impl Fn() -> std::io::Result<usize>, file_size: u64) -> Option<(f64, f64)> {
    println!("  Testing {}...", name);

    // Try to run once first to check if it works
    let start = Instant::now();
    match f() {
        Ok(count) => {
            let elapsed = start.elapsed().as_secs_f64();
            std::hint::black_box(count);

            let throughput = (file_size as f64) / elapsed / 1_000_000_000.0;
            let time_ms = elapsed * 1000.0;

            println!("    {:30} {:>10.2} ms, {:>8.2} GB/s", name, time_ms, throughput);
            Some((throughput, time_ms))
        }
        Err(e) => {
            println!("    {:30} FAILED: {}", name, e);
            None
        }
    }
}

fn get_available_memory() -> u64 {
    // Try to get available memory on macOS
    use std::process::Command;

    if let Ok(output) = Command::new("sysctl")
        .arg("-n")
        .arg("hw.memsize")
        .output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            if let Ok(bytes) = s.trim().parse::<u64>() {
                return bytes;
            }
        }
    }

    // Fallback: assume 16 GB
    16 * 1024 * 1024 * 1024
}

fn main() {
    println!("=== Large File Benchmark (Memory Constrained) ===\n");

    let total_memory = get_available_memory();
    println!("Total system memory: {:.2} GB\n", total_memory as f64 / 1_000_000_000.0);

    // We'll test files that are progressively larger
    // Starting from comfortable sizes up to very large files
    let test_cases = vec![
        ("Comfortable (1M rows, ~50 MB)", 1_000_000, "/tmp/test_large_1m.csv"),
        ("Large (5M rows, ~250 MB)", 5_000_000, "/tmp/test_large_5m.csv"),
        ("Very Large (10M rows, ~500 MB)", 10_000_000, "/tmp/test_large_10m.csv"),
        ("Huge (20M rows, ~1 GB)", 20_000_000, "/tmp/test_large_20m.csv"),
        ("Massive (50M rows, ~2.5 GB)", 50_000_000, "/tmp/test_large_50m.csv"),
    ];

    println!("Note: If in-memory approach fails with OOM, that's expected behavior.\n");
    println!("The buffered approach should handle all sizes gracefully.\n");

    for (desc, num_rows, test_file) in test_cases {
        println!("=== {} ===", desc);

        print!("  Generating file... ");
        std::io::stdout().flush().unwrap();
        let gen_start = Instant::now();
        write_test_file(test_file, num_rows).unwrap();
        let gen_time = gen_start.elapsed().as_secs_f64();
        println!("done in {:.2}s", gen_time);

        let file_size = fs::metadata(test_file).unwrap().len();
        println!("  File size: {:.2} MB ({:.2} GB)",
                 file_size as f64 / 1_000_000.0,
                 file_size as f64 / 1_000_000_000.0);
        println!("  Memory ratio: {:.1}% of total RAM\n",
                 (file_size as f64 / total_memory as f64) * 100.0);

        let disk_result = bench(
            "Disk (4KB buffered)",
            || count_pattern_matches_from_file(test_file, b"Harvard"),
            file_size,
        );

        let mem_result = bench(
            "In-Memory (load all)",
            || count_pattern_matches_in_memory(test_file, b"Harvard"),
            file_size,
        );

        match (disk_result, mem_result) {
            (Some((tp_disk, time_disk)), Some((tp_mem, time_mem))) => {
                let speedup = tp_mem / tp_disk;
                let time_diff = ((time_mem - time_disk) / time_disk) * 100.0;
                println!("  → In-Memory is {:.2}x {} ({:+.1}% time)",
                         if speedup >= 1.0 { speedup } else { 1.0 / speedup },
                         if speedup >= 1.0 { "faster" } else { "slower" },
                         -time_diff);
            }
            (Some(_), None) => {
                println!("  → Disk buffering succeeded, in-memory FAILED (OOM or other error)");
                println!("  → This demonstrates why buffered I/O is essential for large files");
            }
            (None, Some(_)) => {
                println!("  → Unexpected: in-memory succeeded but disk failed");
            }
            (None, None) => {
                println!("  → Both approaches failed");
            }
        }
        println!();

        // Clean up
        let _ = fs::remove_file(test_file);
    }

    println!("\n=== Analysis ===");
    println!("\nBuffered Disk I/O (4KB):");
    println!("  ✓ Constant memory footprint (~4 KB)");
    println!("  ✓ Can handle files of ANY size");
    println!("  ✓ Predictable performance regardless of file size");
    println!("  ✓ No risk of OOM (Out Of Memory) errors");
    println!("  ✓ Can process files larger than available RAM");

    println!("\nIn-Memory (load all):");
    println!("  ✓ Slightly faster for small-medium files (< 500 MB)");
    println!("  ✓ Simpler code");
    println!("  ✗ Memory usage = file size");
    println!("  ✗ Risk of OOM for large files");
    println!("  ✗ May cause system-wide memory pressure");
    println!("  ✗ Cannot handle files larger than available RAM");
    println!("  ✗ Performance degrades if swapping occurs");

    println!("\nRecommendation:");
    println!("  Production systems: Use buffered I/O (reliable, predictable)");
    println!("  Small files (< 100 MB): In-memory is acceptable if memory is plentiful");
    println!("  Large files (> 500 MB): Buffered I/O is essential");
    println!("  Unknown file sizes: Always use buffered I/O (safe default)");

    println!("\nKey Insight:");
    println!("  The blog post's 4KB buffered approach isn't just about performance—");
    println!("  it's about reliability and scalability. It can handle files of any size");
    println!("  with constant memory usage, making it production-ready.");
}
