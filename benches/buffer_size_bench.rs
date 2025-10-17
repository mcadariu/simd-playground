use std::time::Instant;
use std::fs::{self, File};
use std::io::{Write, Read};

const TEST_FILE: &str = "/tmp/test_buffer_size.csv";

fn write_test_file(num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(TEST_FILE)?;
    writeln!(file, "Name,University,Year,GPA,Major")?;

    for i in 0..num_rows {
        writeln!(
            file,
            "Person{},Harvard,{},{:.2},ComputerScience",
            i,
            2020 + (i % 5),
            3.0 + ((i % 10) as f64 / 10.0)
        )?;
    }
    Ok(())
}

fn count_pattern_with_buffer_size(
    file_path: &str,
    pattern: &[u8],
    buffer_size: usize,
) -> std::io::Result<usize> {
    if pattern.is_empty() {
        return Ok(0);
    }

    let mut file = File::open(file_path)?;
    let mut buffer = vec![0u8; buffer_size];
    let mut line_count = 0;
    let mut offset = 0;

    let first_byte = pattern[0];
    let tail_bytes = &pattern[1..];

    loop {
        let bytes_read = file.read(&mut buffer[offset..])? + offset;
        if bytes_read == 0 {
            break;
        }
        offset = 0;

        let mut i = 0;
        while i <= bytes_read.saturating_sub(pattern.len()) {
            match memchr::memchr(first_byte, &buffer[i..bytes_read - pattern.len() + 1]) {
                None => break,
                Some(pos) => {
                    i += pos;
                    if &buffer[i + 1..i + pattern.len()] == tail_bytes {
                        line_count += 1;
                        while i < bytes_read && buffer[i] != b'\n' {
                            i += 1;
                        }
                        i += 1;
                    } else {
                        i += 1;
                    }
                }
            }
        }

        for i in bytes_read.saturating_sub(pattern.len() - 1)..bytes_read {
            if pattern.starts_with(&buffer[i..bytes_read]) {
                let region_len = bytes_read - i;
                buffer.copy_within(i..bytes_read, 0);
                offset = region_len;
                break;
            }
        }
    }

    Ok(line_count)
}

fn bench_buffer_size(buffer_size: usize, iterations: usize, file_size: u64) -> f64 {
    // Warmup
    for _ in 0..5 {
        let _ = count_pattern_with_buffer_size(TEST_FILE, b"Harvard", buffer_size);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        let count = count_pattern_with_buffer_size(TEST_FILE, b"Harvard", buffer_size).unwrap();
        std::hint::black_box(count);
    }
    let elapsed = start.elapsed().as_secs_f64();

    let total_bytes = file_size as f64 * iterations as f64;
    let throughput_gb_s = total_bytes / elapsed / 1_000_000_000.0;

    throughput_gb_s
}

fn main() {
    println!("=== Buffer Size Optimization Benchmark ===\n");

    // Create test file
    println!("Generating test CSV file (200,000 rows)...");
    write_test_file(200_000).expect("Failed to write test file");
    let file_size = fs::metadata(TEST_FILE).unwrap().len();
    println!("File size: {:.2} MB\n", file_size as f64 / 1_000_000.0);

    let iterations = 100;

    // Test different buffer sizes
    let buffer_sizes = vec![
        512,      // 512 B
        1024,     // 1 KB
        2048,     // 2 KB
        4096,     // 4 KB (blog post)
        8192,     // 8 KB
        16384,    // 16 KB
        32768,    // 32 KB
        65536,    // 64 KB
        131072,   // 128 KB
        262144,   // 256 KB
    ];

    println!("{:>12} {:>15} {:>15}", "Buffer Size", "Throughput", "vs 4KB");
    println!("{}", "-".repeat(45));

    let mut results = Vec::new();
    let mut baseline_throughput = 0.0;

    for &size in &buffer_sizes {
        let throughput = bench_buffer_size(size, iterations, file_size);
        results.push((size, throughput));

        if size == 4096 {
            baseline_throughput = throughput;
        }

        let size_str = if size >= 1024 {
            format!("{} KB", size / 1024)
        } else {
            format!("{} B", size)
        };

        let speedup = if baseline_throughput > 0.0 {
            format!("{:+.1}%", (throughput / baseline_throughput - 1.0) * 100.0)
        } else {
            "baseline".to_string()
        };

        println!("{:>12} {:>12.2} GB/s {:>15}", size_str, throughput, speedup);
    }

    // Find optimal
    let optimal = results.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    let optimal_size_str = if optimal.0 >= 1024 {
        format!("{} KB", optimal.0 / 1024)
    } else {
        format!("{} B", optimal.0)
    };

    println!("\n{}", "=".repeat(45));
    println!("Optimal buffer size: {} ({:.2} GB/s)", optimal_size_str, optimal.1);
    println!("Improvement over 4KB: {:.1}%", (optimal.1 / baseline_throughput - 1.0) * 100.0);

    // Clean up
    let _ = fs::remove_file(TEST_FILE);
}
