use std::time::Instant;
use std::fs::{self, File};
use std::io::Write;
use scratchpad::csv_parse_buffer_size_impact::count_pattern_matches_from_file;

fn bench_with_timing(name: &str, f: impl Fn() -> usize, iterations: usize, input_size: usize) -> f64 {
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

fn write_csv_to_file(file_path: &str, num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    let universities = [
        "MIT", "Harvard", "Stanford", "Yale", "Princeton",
        "Columbia", "Cornell", "Brown", "Dartmouth", "Penn"
    ];
    let names = [
        "Alice", "Bob", "Carol", "Dave", "Eve", "Frank", "Grace", "Heidi",
        "Ivan", "Judy", "Mallory", "Olivia", "Peggy", "Rupert", "Sybil", "Trent"
    ];

    writeln!(file, "Name,University,Year,GPA,Major")?;

    for i in 0..num_rows {
        let name = names[i % names.len()];
        let university = universities[i % universities.len()];
        let year = 2020 + (i % 5);
        let gpa = 3.0 + ((i % 10) as f64 / 10.0);
        let major = if i % 3 == 0 { "Computer Science" } else if i % 3 == 1 { "Mathematics" } else { "Physics" };

        writeln!(file, "{},{},{},{:.2},{}", name, university, year, gpa, major)?;
    }

    Ok(())
}

fn main() {
    println!("=== CSV Pattern Matching Benchmarks (Blog Post Method) ===\n");
    println!("Matches the blog post exactly:");
    println!("  - 4KB fixed buffer");
    println!("  - memchr (like Array.IndexOf) to find first byte");
    println!("  - Check if tail bytes match");
    println!("  - Handle buffer boundary with offset\n");

    // Generate test CSV file
    let test_file = "/tmp/test_researchers.csv";
    println!("Generating test CSV file...");
    write_csv_to_file(test_file, 200_000).expect("Failed to write test file");

    let file_size = fs::metadata(test_file).unwrap().len();
    println!("Test file created: {}", test_file);
    println!("File size: {:.2} MB\n", file_size as f64 / 1_000_000.0);

    let iterations = 100;

    // Test 1: Small file
    println!("--- Small file (1,000 rows, ~50 KB) ---");
    let small_file = "/tmp/test_small.csv";
    write_csv_to_file(small_file, 1_000).expect("Failed to write small file");
    let small_size = fs::metadata(small_file).unwrap().len();

    bench_with_timing(
        "Disk (4KB buf + memchr)",
        || count_pattern_matches_from_file(small_file, b"Harvard").unwrap(),
        iterations * 10,
        small_size as usize,
    );
    println!();

    let _ = fs::remove_file(small_file);

    // Test 2: Medium file
    println!("--- Medium file (10,000 rows, ~500 KB) ---");
    let medium_file = "/tmp/test_medium.csv";
    write_csv_to_file(medium_file, 10_000).expect("Failed to write medium file");
    let medium_size = fs::metadata(medium_file).unwrap().len();

    bench_with_timing(
        "Disk (4KB buf + memchr)",
        || count_pattern_matches_from_file(medium_file, b"Harvard").unwrap(),
        iterations * 2,
        medium_size as usize,
    );
    println!();

    let _ = fs::remove_file(medium_file);

    // Test 3: Large file (similar to blog post: 11 MB)
    println!("--- Large file (200,000 rows, ~7 MB) ---");
    println!("  (Blog post used 217,096 rows, 11 MB)\n");

    bench_with_timing(
        "Disk (4KB buf + memchr)",
        || count_pattern_matches_from_file(test_file, b"Harvard").unwrap(),
        iterations,
        file_size as usize,
    );
    println!();

    // Test 4: Different pattern lengths
    println!("--- Different pattern lengths (200,000 rows) ---");
    let patterns = vec![
        (b"H" as &[u8], "Single char"),
        (b"MIT" as &[u8], "3 chars"),
        (b"Harvard" as &[u8], "7 chars"),
        (b"Computer Science" as &[u8], "16 chars"),
    ];

    for (pattern, desc) in patterns {
        println!("  Pattern: {} ({})", desc, std::str::from_utf8(pattern).unwrap());

        bench_with_timing(
            "    memchr",
            || count_pattern_matches_from_file(test_file, pattern).unwrap(),
            iterations / 2,
            file_size as usize,
        );

        println!();
    }

    // Clean up
    let _ = fs::remove_file(test_file);

    println!("\n=== Summary ===");
    println!("Blog post reference (C# on 11 MB file, disk-based):");
    println!("  Line scanning:         1.1 GB/s");
    println!("  CsvHelper library:     0.28 GB/s");
    println!("  NReco.Csv library:     0.33 GB/s");
    println!("  Sep library:           0.64 GB/s");
    println!("  Low-level byte search: 3.5 GB/s");
    println!("\nOur Rust implementation (memchr-based, 4KB buffers):");
    println!("  Uses memchr (Rust's optimized equivalent to Array.IndexOf)");
    println!("  Reads from disk with 4KB fixed buffers");
    println!("  Handles buffer boundaries with offset mechanism");
}
