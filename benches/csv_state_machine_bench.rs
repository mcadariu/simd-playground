use std::time::Instant;
use std::fs::{self, File};
use std::io::Write;
use scratchpad::csv_state_machine::{parse_csv_state_machine, parse_csv_if_else};

fn bench_with_timing(name: &str, f: impl Fn() -> (usize, usize), iterations: usize, input_size: usize) -> f64 {
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
        "{:35} {:.2} ms total, {:.2} GB/s throughput",
        format!("{}:", name),
        elapsed_secs * 1000.0,
        throughput_gb_s
    );

    throughput_gb_s
}

fn write_csv_to_file(file_path: &str, num_rows: usize, include_quotes: bool, include_embedded: bool) -> std::io::Result<()> {
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

        if include_quotes && include_embedded && i % 10 == 0 {
            // Include some fields with embedded commas and newlines
            writeln!(file, "\"{}\",\"{}, USA\",{},{:.2},\"{}\"", name, university, year, gpa, major)?;
        } else if include_quotes {
            writeln!(file, "\"{}\",\"{}\",{},{:.2},\"{}\"", name, university, year, gpa, major)?;
        } else {
            writeln!(file, "{},{},{},{:.2},{}", name, university, year, gpa, major)?;
        }
    }

    Ok(())
}

fn main() {
    println!("=== CSV Parsing Benchmarks: State Machine vs If/Else ===\n");
    println!("Based on KWIllets' comment on Lemire's blog:");
    println!("https://lemire.me/blog/2008/12/19/parsing-csv-files-is-cpu-bound-a-c-test-case-update-2/\n");
    println!("Comparison:");
    println!("  STATE MACHINE: Table-driven DFA with minimal branching");
    println!("  IF/ELSE:       Simple conditional logic (many branches)\n");

    let iterations = 100;

    // Test 1: Simple CSV (no quotes)
    println!("--- Test 1: Simple CSV (10,000 rows, no quotes) ---");
    let simple_file = "/tmp/test_simple_csv.csv";
    write_csv_to_file(simple_file, 10_000, false, false).expect("Failed to write simple file");
    let simple_data = fs::read(simple_file).unwrap();
    let simple_size = simple_data.len();
    println!("File size: {:.2} KB\n", simple_size as f64 / 1_000.0);

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&simple_data),
        iterations * 2,
        simple_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&simple_data),
        iterations * 2,
        simple_size,
    );

    println!("Speedup: {:.2}x\n", sm_throughput / ie_throughput);
    let _ = fs::remove_file(simple_file);

    // Test 2: CSV with quoted fields
    println!("--- Test 2: CSV with quoted fields (10,000 rows) ---");
    let quoted_file = "/tmp/test_quoted_csv.csv";
    write_csv_to_file(quoted_file, 10_000, true, false).expect("Failed to write quoted file");
    let quoted_data = fs::read(quoted_file).unwrap();
    let quoted_size = quoted_data.len();
    println!("File size: {:.2} KB\n", quoted_size as f64 / 1_000.0);

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&quoted_data),
        iterations * 2,
        quoted_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&quoted_data),
        iterations * 2,
        quoted_size,
    );

    println!("Speedup: {:.2}x\n", sm_throughput / ie_throughput);
    let _ = fs::remove_file(quoted_file);

    // Test 3: CSV with embedded commas in quotes
    println!("--- Test 3: CSV with embedded commas (10,000 rows) ---");
    let embedded_file = "/tmp/test_embedded_csv.csv";
    write_csv_to_file(embedded_file, 10_000, true, true).expect("Failed to write embedded file");
    let embedded_data = fs::read(embedded_file).unwrap();
    let embedded_size = embedded_data.len();
    println!("File size: {:.2} KB\n", embedded_size as f64 / 1_000.0);

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&embedded_data),
        iterations,
        embedded_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&embedded_data),
        iterations,
        embedded_size,
    );

    println!("Speedup: {:.2}x\n", sm_throughput / ie_throughput);
    let _ = fs::remove_file(embedded_file);

    // Test 4: Large file
    println!("--- Test 4: Large CSV (200,000 rows, with quotes) ---");
    let large_file = "/tmp/test_large_csv.csv";
    write_csv_to_file(large_file, 200_000, true, false).expect("Failed to write large file");
    let large_data = fs::read(large_file).unwrap();
    let large_size = large_data.len();
    println!("File size: {:.2} MB\n", large_size as f64 / 1_000_000.0);

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&large_data),
        50,
        large_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&large_data),
        50,
        large_size,
    );

    println!("Speedup: {:.2}x\n", sm_throughput / ie_throughput);
    let _ = fs::remove_file(large_file);

    // Test 5: Different file sizes
    println!("--- Test 5: Scaling with file size (quoted CSV) ---");
    let sizes = vec![
        (100, "100 rows (~5 KB)", 1000),
        (1_000, "1K rows (~50 KB)", 500),
        (10_000, "10K rows (~500 KB)", 200),
        (50_000, "50K rows (~2.5 MB)", 50),
    ];

    for (num_rows, desc, iter) in sizes {
        println!("\n  {}", desc);
        let test_file = format!("/tmp/test_size_{}.csv", num_rows);
        write_csv_to_file(&test_file, num_rows, true, false).expect("Failed to write file");
        let data = fs::read(&test_file).unwrap();
        let size = data.len();

        let sm_throughput = bench_with_timing(
            "    State Machine",
            || parse_csv_state_machine(&data),
            iter,
            size,
        );

        let ie_throughput = bench_with_timing(
            "    If/Else",
            || parse_csv_if_else(&data),
            iter,
            size,
        );

        println!("    Speedup: {:.2}x", sm_throughput / ie_throughput);
        let _ = fs::remove_file(&test_file);
    }

    println!("\n=== Summary ===");
    println!("\nKey Insights:");
    println!("  1. State machine has fewer branches -> fewer CPU mispredictions");
    println!("  2. Table-driven transitions are cache-friendly");
    println!("  3. Sentinel padding eliminates boundary checks in the hot loop");
    println!("  4. If/else is simpler to write but slower due to branch overhead");
    println!("\nFrom KWIllets' comment:");
    println!("  'The two biggest factors are low instruction count and low branch count.'");
    println!("  'Branches cause pipeline stalls when mispredicted.'");
    println!("\nThe state machine approach demonstrates the power of:");
    println!("  - Minimizing conditional branches");
    println!("  - Using lookup tables instead of if/else chains");
    println!("  - Structuring data flow to be cache-friendly");
}
