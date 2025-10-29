use std::time::Instant;
use std::fs::{self, File};
use std::io::Write;
use scratchpad::csv_state_machine::{
    parse_csv_state_machine, parse_csv_state_machine_no_copy,
    parse_csv_state_machine_branchless, parse_csv_if_else
};

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

/// Generate predictable CSV (normal case - good for branch prediction)
fn write_predictable_csv(file_path: &str, num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "Name,University,Year,GPA,Major")?;

    for i in 0..num_rows {
        writeln!(
            file,
            "Alice,\"Harvard University\",{},{:.2},\"Computer Science\"",
            2020 + (i % 5),
            3.0 + ((i % 10) as f64 / 10.0)
        )?;
    }

    Ok(())
}

/// Generate adversarial CSV designed to defeat branch prediction
fn write_adversarial_csv(file_path: &str, num_rows: usize, seed: u64) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "Name,University,Year,GPA,Major")?;

    // Simple LCG for reproducible "randomness"
    let mut rng = seed;
    let mut next_random = || {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        rng
    };

    for _ in 0..num_rows {
        let pattern = next_random() % 8;

        match pattern {
            0 => {
                // Mix of quoted and unquoted, with embedded commas
                writeln!(file, "\"A,B\",C,2020,3.5,\"X,Y\"")?;
            }
            1 => {
                // Escaped quotes (triggers QuoteInQuoted state frequently)
                writeln!(file, "\"Alice\"\"Bob\",\"Har\"\"vard\",2021,3.6,CS")?;
            }
            2 => {
                // Very short fields (lots of state transitions)
                writeln!(file, "A,B,C,D,E")?;
            }
            3 => {
                // Alternating quoted/unquoted per field
                writeln!(file, "\"A\",B,\"C\",D,\"E\"")?;
            }
            4 => {
                // Empty fields mixed with quoted
                writeln!(file, ",\"Harvard\",,3.7,")?;
            }
            5 => {
                // Long quoted field with embedded newlines
                writeln!(file, "\"Line1\nLine2\",MIT,2022,3.8,\"Math\nPhysics\"")?;
            }
            6 => {
                // Mix everything: commas, quotes, newlines in quotes
                writeln!(file, "\"A,B\nC\",\"D\"\"E\",2023,3.9,\"F,G\nH\"")?;
            }
            7 => {
                // Normal line to keep it somewhat valid
                writeln!(file, "Normal,Harvard,2024,4.0,Engineering")?;
            }
            _ => unreachable!(),
        }
    }

    Ok(())
}

/// Generate random-looking CSV with unpredictable patterns
fn write_random_pattern_csv(file_path: &str, num_rows: usize, seed: u64) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "A,B,C,D,E")?;

    let mut rng = seed;
    let mut next_random = || {
        rng = rng.wrapping_mul(1103515245).wrapping_add(12345);
        (rng >> 16) as u8
    };

    for _ in 0..num_rows {
        for field_idx in 0..5 {
            if field_idx > 0 {
                write!(file, ",")?;
            }

            let r = next_random();

            // Randomly decide: quoted, unquoted, empty, with special chars
            if r < 50 {
                // Empty field
            } else if r < 100 {
                // Simple unquoted
                write!(file, "X{}", r)?;
            } else if r < 150 {
                // Quoted with comma
                write!(file, "\"A,{}\"", r)?;
            } else if r < 200 {
                // Quoted with escaped quote
                write!(file, "\"A\"\"{}\"", r)?;
            } else {
                // Quoted with newline
                write!(file, "\"A\n{}\"", r)?;
            }
        }
        writeln!(file)?;
    }

    Ok(())
}

/// Generate CSV with alternating patterns (worst for branch prediction)
fn write_alternating_csv(file_path: &str, num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "A,B,C")?;

    for i in 0..num_rows {
        // Alternate between completely different patterns
        if i % 2 == 0 {
            writeln!(file, "\"A,B,C\",\"D\nE\",\"F\"\"G\"")?;
        } else {
            writeln!(file, "Simple,Normal,Field")?;
        }
    }

    Ok(())
}

/// Generate CSV with many short fields (maximize state transitions)
fn write_short_fields_csv(file_path: &str, num_rows: usize) -> std::io::Result<()> {
    let mut file = File::create(file_path)?;
    writeln!(file, "A,B,C,D,E,F,G,H,I,J,K,L,M,N,O,P,Q,R,S,T")?;

    for i in 0..num_rows {
        // 20 fields, mix quoted and unquoted
        for j in 0..20 {
            if j > 0 {
                write!(file, ",")?;
            }
            // Alternate to create unpredictable pattern
            if (i + j) % 3 == 0 {
                write!(file, "\"{}\"", j)?;
            } else if (i + j) % 3 == 1 {
                write!(file, "{}", j)?;
            } else {
                // Empty field
            }
        }
        writeln!(file)?;
    }

    Ok(())
}

fn main() {
    println!("=== Adversarial CSV Benchmarks: When State Machines Win ===\n");
    println!("Goal: Create CSV patterns that defeat branch prediction.");
    println!("State machine should win when branches become unpredictable.\n");

    let iterations = 100;

    // Baseline: Predictable CSV
    println!("--- Baseline: Predictable CSV (10,000 rows) ---");
    println!("(Same structure every row - branch predictor loves this)\n");
    let predictable_file = "/tmp/test_predictable.csv";
    write_predictable_csv(predictable_file, 10_000).expect("Failed to write file");
    let predictable_data = fs::read(predictable_file).unwrap();
    let predictable_size = predictable_data.len();

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&predictable_data),
        iterations,
        predictable_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&predictable_data),
        iterations,
        predictable_size,
    );

    println!("If/Else advantage: {:.2}x faster\n", ie_throughput / sm_throughput);
    let _ = fs::remove_file(predictable_file);

    // Test 1: Adversarial CSV with random patterns
    println!("--- Test 1: Adversarial CSV (10,000 rows) ---");
    println!("(8 different patterns pseudo-randomly mixed)\n");
    let adversarial_file = "/tmp/test_adversarial.csv";
    write_adversarial_csv(adversarial_file, 10_000, 12345).expect("Failed to write file");
    let adversarial_data = fs::read(adversarial_file).unwrap();
    let adversarial_size = adversarial_data.len();

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&adversarial_data),
        iterations,
        adversarial_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&adversarial_data),
        iterations,
        adversarial_size,
    );

    let ratio = sm_throughput / ie_throughput;
    if ratio > 1.0 {
        println!("State Machine advantage: {:.2}x faster ✓\n", ratio);
    } else {
        println!("If/Else still faster: {:.2}x\n", ie_throughput / sm_throughput);
    }
    let _ = fs::remove_file(adversarial_file);

    // Test 2: Random pattern CSV
    println!("--- Test 2: Random Pattern CSV (10,000 rows) ---");
    println!("(Randomly generated field types - very unpredictable)\n");
    let random_file = "/tmp/test_random.csv";
    write_random_pattern_csv(random_file, 10_000, 54321).expect("Failed to write file");
    let random_data = fs::read(random_file).unwrap();
    let random_size = random_data.len();

    let sm_throughput = bench_with_timing(
        "State Machine (with copy)",
        || parse_csv_state_machine(&random_data),
        iterations,
        random_size,
    );

    let sm_no_copy_throughput = bench_with_timing(
        "State Machine (no copy)",
        || parse_csv_state_machine_no_copy(&random_data),
        iterations,
        random_size,
    );

    let sm_branchless_throughput = bench_with_timing(
        "State Machine (branchless)",
        || parse_csv_state_machine_branchless(&random_data),
        iterations,
        random_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&random_data),
        iterations,
        random_size,
    );

    let ratio = sm_branchless_throughput / ie_throughput;
    if ratio > 1.0 {
        println!("\n🎉 State Machine (branchless) WINS: {:.2}x faster ✓", ratio);
    } else {
        println!("\nIf/Else still faster: {:.2}x", ie_throughput / sm_branchless_throughput);
    }
    println!("Branchless improvement: {:.2}x vs copy, {:.2}x vs no-copy\n",
        sm_branchless_throughput / sm_throughput,
        sm_branchless_throughput / sm_no_copy_throughput);
    let _ = fs::remove_file(random_file);

    // Test 3: Alternating patterns (pathological for branch prediction)
    println!("--- Test 3: Alternating Patterns (10,000 rows) ---");
    println!("(Row-by-row alternation - defeats pattern recognition)\n");
    let alternating_file = "/tmp/test_alternating.csv";
    write_alternating_csv(alternating_file, 10_000).expect("Failed to write file");
    let alternating_data = fs::read(alternating_file).unwrap();
    let alternating_size = alternating_data.len();

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&alternating_data),
        iterations,
        alternating_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&alternating_data),
        iterations,
        alternating_size,
    );

    let ratio = sm_throughput / ie_throughput;
    if ratio > 1.0 {
        println!("State Machine advantage: {:.2}x faster ✓\n", ratio);
    } else {
        println!("If/Else still faster: {:.2}x\n", ie_throughput / sm_throughput);
    }
    let _ = fs::remove_file(alternating_file);

    // Test 4: Many short fields (maximize state transitions)
    println!("--- Test 4: Many Short Fields (10,000 rows, 20 fields each) ---");
    println!("(Maximize state transitions - more work per byte)\n");
    let short_fields_file = "/tmp/test_short_fields.csv";
    write_short_fields_csv(short_fields_file, 10_000).expect("Failed to write file");
    let short_fields_data = fs::read(short_fields_file).unwrap();
    let short_fields_size = short_fields_data.len();

    let sm_throughput = bench_with_timing(
        "State Machine",
        || parse_csv_state_machine(&short_fields_data),
        iterations,
        short_fields_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&short_fields_data),
        iterations,
        short_fields_size,
    );

    let ratio = sm_throughput / ie_throughput;
    if ratio > 1.0 {
        println!("State Machine advantage: {:.2}x faster ✓\n", ratio);
    } else {
        println!("If/Else still faster: {:.2}x\n", ie_throughput / sm_throughput);
    }
    let _ = fs::remove_file(short_fields_file);

    // Test 5: Scaling - larger adversarial file
    println!("--- Test 5: Large Adversarial CSV (100,000 rows) ---");
    println!("(Testing if advantage grows with larger unpredictable data)\n");
    let large_adversarial_file = "/tmp/test_large_adversarial.csv";
    write_adversarial_csv(large_adversarial_file, 100_000, 99999).expect("Failed to write file");
    let large_adversarial_data = fs::read(large_adversarial_file).unwrap();
    let large_adversarial_size = large_adversarial_data.len();
    println!("File size: {:.2} MB\n", large_adversarial_size as f64 / 1_000_000.0);

    let sm_throughput = bench_with_timing(
        "State Machine (with copy)",
        || parse_csv_state_machine(&large_adversarial_data),
        20,
        large_adversarial_size,
    );

    let sm_no_copy_throughput = bench_with_timing(
        "State Machine (no copy)",
        || parse_csv_state_machine_no_copy(&large_adversarial_data),
        20,
        large_adversarial_size,
    );

    let ie_throughput = bench_with_timing(
        "If/Else",
        || parse_csv_if_else(&large_adversarial_data),
        20,
        large_adversarial_size,
    );

    let ratio = sm_no_copy_throughput / ie_throughput;
    if ratio > 1.0 {
        println!("State Machine (no copy) advantage: {:.2}x faster ✓", ratio);
    } else {
        println!("If/Else still faster: {:.2}x", ie_throughput / sm_no_copy_throughput);
    }
    println!("No-copy improvement: {:.2}x faster than copy version\n", sm_no_copy_throughput / sm_throughput);
    let _ = fs::remove_file(large_adversarial_file);

    println!("\n=== Summary ===");
    println!("\nWhen Branch Prediction Fails:");
    println!("  - Unpredictable data patterns reduce if/else efficiency");
    println!("  - State machine has consistent performance (table lookup cost is fixed)");
    println!("  - Many state transitions per byte amplify the difference");
    println!("\nHowever, Modern CPUs are Remarkably Good:");
    println!("  - Even 'random' patterns often have hidden predictability");
    println!("  - Branch target buffers and pattern history tables are sophisticated");
    println!("  - Memory access patterns can matter more than branch count");
    println!("\nReal-world takeaway:");
    println!("  - For typical CSV: if/else wins (predictable structure)");
    println!("  - For adversarial/random: state machine may win");
    println!("  - Always profile with YOUR actual data!");
}
