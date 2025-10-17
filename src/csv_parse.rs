//! Fast CSV pattern matching using byte-level search with disk I/O.
//!
//! Based on: https://lemire.me/blog/2024/10/17/how-fast-can-you-parse-a-csv-file-in-c/
//!
//! This implementation matches the blog post's approach:
//! - Read CSV files from disk using 4KB fixed buffers
//! - Use memchr (like Array.IndexOf) to find the first byte of the pattern
//! - Check if remaining bytes match
//! - Handle patterns spanning buffer boundaries
//!
//! Key insight: Using memchr to jump to candidates is 12x faster than parsing CSV fields.
//!
//! WARNING: This prioritizes speed over correctness. Does NOT handle:
//! - Quoted fields with embedded newlines
//! - Escaped quotes
//! - Multi-byte encodings

use std::fs::File;
use std::io::{self, Read};

const BUFFER_SIZE: usize = 4096;

/// Count lines containing a pattern by reading from disk with 4KB buffering.
///
/// Matches the blog post's C# implementation:
/// ```csharp
/// i = Array.IndexOf(buffer, (byte)harvardBytes[0], i, ...);
/// if (region.SequenceEqual(tailbytes)) { ... }
/// ```
///
/// # Example
/// ```no_run
/// use scratchpad::csv_parse::count_pattern_matches_from_file;
///
/// let count = count_pattern_matches_from_file("researchers.csv", b"Harvard")
///     .expect("Failed to read file");
/// println!("Found {} matching lines", count);
/// ```
pub fn count_pattern_matches_from_file(
    file_path: &str,
    pattern: &[u8],
) -> io::Result<usize> {
    if pattern.is_empty() {
        return Ok(0);
    }

    let mut file = File::open(file_path)?;
    let mut buffer = vec![0u8; BUFFER_SIZE];
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

        // Search for pattern in current buffer
        let mut i = 0;
        while i <= bytes_read.saturating_sub(pattern.len()) {
            // Find first byte using memchr (like Array.IndexOf)
            match memchr::memchr(first_byte, &buffer[i..bytes_read - pattern.len() + 1]) {
                None => break,
                Some(pos) => {
                    i += pos;

                    // Check if tail bytes match (like region.SequenceEqual)
                    if &buffer[i + 1..i + pattern.len()] == tail_bytes {
                        line_count += 1;

                        // Skip to end of line to avoid double-counting
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

        // Handle pattern spanning buffer boundary
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

/// Count lines containing a pattern by loading entire file into memory first.
///
/// This is the simpler approach: read everything, then search.
/// Trades memory for simplicity.
pub fn count_pattern_matches_in_memory(
    file_path: &str,
    pattern: &[u8],
) -> io::Result<usize> {
    if pattern.is_empty() {
        return Ok(0);
    }

    // Load entire file into memory
    let data = std::fs::read(file_path)?;

    let first_byte = pattern[0];
    let tail_bytes = &pattern[1..];
    let mut line_count = 0;
    let mut i = 0;

    // Search through the data
    while i <= data.len().saturating_sub(pattern.len()) {
        match memchr::memchr(first_byte, &data[i..]) {
            None => break,
            Some(pos) => {
                i += pos;

                if i + pattern.len() <= data.len() && &data[i + 1..i + pattern.len()] == tail_bytes {
                    line_count += 1;

                    // Skip to end of line
                    while i < data.len() && data[i] != b'\n' {
                        i += 1;
                    }
                    i += 1;
                } else {
                    i += 1;
                }
            }
        }
    }

    Ok(line_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn create_test_file(path: &str, content: &[u8]) -> io::Result<()> {
        File::create(path)?.write_all(content)
    }

    #[test]
    fn test_basic() {
        let file = "/tmp/test_csv_basic.csv";
        let content = b"Name,University,Year\nAlice,MIT,2020\nBob,Harvard,2021\nCarol,Harvard,2022\n";

        create_test_file(file, content).unwrap();
        let count = count_pattern_matches_from_file(file, b"Harvard").unwrap();

        assert_eq!(count, 2);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn test_no_match() {
        let file = "/tmp/test_csv_no_match.csv";
        let content = b"Name,University,Year\nAlice,MIT,2020\n";

        create_test_file(file, content).unwrap();
        let count = count_pattern_matches_from_file(file, b"Harvard").unwrap();

        assert_eq!(count, 0);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn test_buffer_boundary() {
        let file = "/tmp/test_csv_boundary.csv";
        let mut content = Vec::new();

        // Fill buffer close to 4KB
        for _ in 0..800 {
            content.extend_from_slice(b"Name,MIT,2020\n");
        }
        content.extend_from_slice(b"Bob,Harvard,2021\n");

        create_test_file(file, &content).unwrap();
        let count = count_pattern_matches_from_file(file, b"Harvard").unwrap();

        assert_eq!(count, 1);
        let _ = std::fs::remove_file(file);
    }

    #[test]
    fn test_multiple_matches_same_line() {
        let file = "/tmp/test_csv_multi.csv";
        let content = b"Name,University,Year\nHarvard,Harvard University,2020\n";

        create_test_file(file, content).unwrap();
        let count = count_pattern_matches_from_file(file, b"Harvard").unwrap();

        assert_eq!(count, 1); // Should count line once, not twice
        let _ = std::fs::remove_file(file);
    }
}
