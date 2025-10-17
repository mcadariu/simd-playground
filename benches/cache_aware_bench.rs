use std::time::Instant;
use std::fs::{self, File};
use std::io::{Write, Read};

const TEST_FILE: &str = "/tmp/test_cache_aware.csv";

fn write_test_file(num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(TEST_FILE)?;
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

fn count_with_buffer(file_path: &str, pattern: &[u8], buffer_size: usize) -> std::io::Result<usize> {
    let mut file = File::open(file_path)?;
    let mut buffer = vec![0u8; buffer_size];
    let mut line_count = 0;
    let mut offset = 0;

    let first_byte = pattern[0];
    let tail_bytes = &pattern[1..];

    loop {
        let bytes_read = file.read(&mut buffer[offset..])? + offset;
        if bytes_read == 0 { break; }
        offset = 0;

        let mut i = 0;
        while i <= bytes_read.saturating_sub(pattern.len()) {
            match memchr::memchr(first_byte, &buffer[i..bytes_read - pattern.len() + 1]) {
                None => break,
                Some(pos) => {
                    i += pos;
                    if &buffer[i + 1..i + pattern.len()] == tail_bytes {
                        line_count += 1;
                        while i < bytes_read && buffer[i] != b'\n' { i += 1; }
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
            }
        }

        for i in bytes_read.saturating_sub(pattern.len() - 1)..bytes_read {
            if pattern.starts_with(&buffer[i..bytes_read]) {
                buffer.copy_within(i..bytes_read, 0);
                offset = bytes_read - i;
                break;
            }
        }
    }
    Ok(line_count)
}

fn bench(buffer_size: usize, iterations: usize, file_size: u64) -> (f64, f64) {
    // Warmup
    for _ in 0..5 {
        let _ = count_with_buffer(TEST_FILE, b"Harvard", buffer_size);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let count = count_with_buffer(TEST_FILE, b"Harvard", buffer_size).unwrap();
        std::hint::black_box(count);
    }
    let elapsed = start.elapsed().as_secs_f64();

    let total_bytes = file_size as f64 * iterations as f64;
    let throughput = total_bytes / elapsed / 1_000_000_000.0;
    let time_per_op_us = (elapsed / iterations as f64) * 1_000_000.0;

    (throughput, time_per_op_us)
}

fn main() {
    println!("=== Cache-Aware Buffer Size Analysis ===\n");

    // Get cache info
    println!("CPU Cache Architecture (ARM M-series):");
    println!("  L1 Data Cache:  64 KB  (per P-core)");
    println!("  L1 Data Cache: 128 KB  (per E-core)");
    println!("  L2 Cache:        4 MB  (shared)\n");

    println!("Generating test file...");
    write_test_file(200_000).unwrap();
    let file_size = fs::metadata(TEST_FILE).unwrap().len();
    println!("File size: {:.2} MB\n", file_size as f64 / 1_000_000.0);

    let iterations = 100;

    // Test buffer sizes around cache boundaries
    let test_configs = vec![
        // Sub-L1 cache sizes
        ("1 KB", 1024, "Much smaller than L1"),
        ("4 KB", 4096, "Blog post (page size)"),
        ("8 KB", 8192, ""),
        ("16 KB", 16384, "1/4 of L1"),
        ("32 KB", 32768, "1/2 of L1"),

        // Around L1 boundary (64-128 KB)
        ("48 KB", 49152, "3/4 of L1"),
        ("64 KB", 65536, "⚠️  L1 boundary (P-core)"),
        ("80 KB", 81920, "Just above L1 (P-core)"),
        ("96 KB", 98304, ""),
        ("112 KB", 114688, ""),
        ("128 KB", 131072, "⚠️  L1 boundary (E-core)"),
        ("160 KB", 163840, "Just above L1"),

        // L2 but > L1
        ("192 KB", 196608, ""),
        ("256 KB", 262144, "Previous optimal"),
        ("512 KB", 524288, ""),
    ];

    println!("{:>10} {:>15} {:>12} {:>12} {}",
             "Buffer", "Throughput", "Time/Op", "vs 4KB", "Notes");
    println!("{}", "=".repeat(80));

    let mut results = Vec::new();
    let mut baseline_throughput = 0.0;

    for (name, size, note) in test_configs {
        let (throughput, time_us) = bench(size, iterations, file_size);
        results.push((name, size, throughput, time_us));

        if size == 4096 {
            baseline_throughput = throughput;
        }

        let speedup = if baseline_throughput > 0.0 {
            format!("{:+.1}%", (throughput / baseline_throughput - 1.0) * 100.0)
        } else {
            "".to_string()
        };

        println!("{:>10} {:>12.2} GB/s {:>9.1} μs {:>12} {}",
                 name, throughput, time_us, speedup, note);
    }

    // Analysis
    let optimal = results.iter().max_by(|a, b| a.2.partial_cmp(&b.2).unwrap()).unwrap();

    println!("\n{}", "=".repeat(80));
    println!("Analysis:");
    println!("  Optimal: {} ({:.2} GB/s, {:.1}% faster than 4KB)",
             optimal.0, optimal.2, (optimal.2 / baseline_throughput - 1.0) * 100.0);

    // Find L1 cache boundary performance
    let l1_64kb = results.iter().find(|r| r.1 == 65536).unwrap();
    let l1_128kb = results.iter().find(|r| r.1 == 131072).unwrap();

    println!("\n  L1 Cache Boundary Effects:");
    println!("    64 KB (P-core L1):  {:.2} GB/s", l1_64kb.2);
    println!("    128 KB (E-core L1): {:.2} GB/s", l1_128kb.2);

    if optimal.1 <= 65536 {
        println!("\n  ✓ Optimal buffer fits entirely in L1 cache (P-core)");
    } else if optimal.1 <= 131072 {
        println!("\n  ⚠ Optimal buffer fits in L1 on E-cores, but not P-cores");
    } else {
        println!("\n  ⚠ Optimal buffer exceeds L1, relies on L2 cache");
    }

    println!("\nConclusion:");
    if optimal.1 <= 65536 {
        println!("  Buffer size ≤ 64 KB keeps data in L1 cache (fastest access)");
        println!("  This explains the performance plateau around 64 KB");
    } else {
        println!("  Larger buffers ({}) perform better despite exceeding L1", optimal.0);
        println!("  Benefit of fewer syscalls outweighs L1 cache misses");
        println!("  L2 cache (4 MB) is still fast enough");
    }

    let _ = fs::remove_file(TEST_FILE);
}
