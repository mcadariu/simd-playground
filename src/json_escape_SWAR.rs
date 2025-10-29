//! Detect JSON escapable characters efficiently using SWAR.
//!
//! This module detects characters that need escaping in JSON strings:
//! - Control characters (bytes 0-31)
//! - Quote character (byte 34, '"')
//! - Backslash character (byte 92, '\')
//!
//! Based on: https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/

// ═══════════════════════════════════════════════════════════════════════════
//                    SWAR: SIMD Within A Register
// ═══════════════════════════════════════════════════════════════════════════
//
// SWAR processes 8 bytes in parallel within a single 64-bit register.
// It uses bitwise operations to detect specific byte patterns simultaneously.
//
// Example: Checking if any byte in [10, 34, 92, 65] needs JSON escaping
//
//   Register: 0x000000005A225C0A (in hex)
//                       ^  ^  ^  ^
//                       |  |  |  |
//              Byte 3: 90  |  |  |
//              Byte 2: 34 (")  |  |
//              Byte 1: 92 (\)  |
//              Byte 0: 10 (control)
//
// All checks happen in parallel using bitwise operations!

//=== JSON Escape Detection Benchmarks (SWAR) ===
//
// --- Clean ASCII (no escapable chars) ---
// Scalar (clean, 1 MB):          0.00 ms total, 444444.44 GB/s throughput
// SWAR (clean, 1 MB):            0.00 ms total, 307692.31 GB/s throughput
//
// --- With escapable chars (early detection) ---
// Scalar (early escape, 1 MB):   0.06 ms total, 15564.20 GB/s throughput
// SWAR (early escape, 1 MB):     0.03 ms total, 29375.48 GB/s throughput
//
// --- Mixed content (quotes, backslashes, newlines) ---
// Scalar (mixed, 1 MB):          0.01 ms total, 132590.82 GB/s throughput
// SWAR (mixed, 1 MB):            0.01 ms total, 175162.02 GB/s throughput
//
// --- Different input sizes (clean ASCII) ---
//   1 KB:
//     Scalar:                    0.00 ms total, 444.19 GB/s throughput
//     SWAR:                      0.00 ms total, 307.52 GB/s throughput
//
//   10 KB:
//     Scalar:                    0.00 ms total, 3973.12 GB/s throughput
//     SWAR:                      0.00 ms total, 2982.82 GB/s throughput
//
//   100 KB:
//     Scalar:                    0.00 ms total, inf GB/s throughput
//     SWAR:                      0.00 ms total, 24975.61 GB/s throughput
//
//   1000 KB:
//     Scalar:                    0.00 ms total, 243809.52 GB/s throughput
//     SWAR:                      0.00 ms total, 243809.52 GB/s throughput
//
// --- Very large input (10 MB, clean ASCII) ---
// Scalar (10 MB):                0.00 ms total, 3436426.12 GB/s throughput
// SWAR (10 MB):                  0.00 ms total, 3003003.00 GB/s throughput
//
// --- Worst case (escapable at end) ---
// Scalar (worst case, 1 MB):     629.03 ms total, 1.59 GB/s throughput
// SWAR (worst case, 1 MB):       336.55 ms total, 2.97 GB/s throughput

// ───────────────────────────────────────────────────────────────────────────
//                         Scalar Reference
// ───────────────────────────────────────────────────────────────────────────

/// Check if a single byte needs JSON escaping (scalar version).
///
/// Returns true if the byte is:
/// - A control character (0-31)
/// - A quote (34, '"')
/// - A backslash (92, '\')
#[inline]
pub fn needs_json_escape_scalar(byte: u8) -> bool {
    byte < 32 || byte == 34 || byte == 92
}

/// Check if any byte in a buffer needs JSON escaping (scalar version).
pub fn has_json_escapable_byte_scalar(buffer: &[u8]) -> bool {
    buffer.iter().any(|&b| needs_json_escape_scalar(b))
}

// ═══════════════════════════════════════════════════════════════════════════
//                    SWAR: Parallel Detection in 64 bits
// ═══════════════════════════════════════════════════════════════════════════
//
// The core SWAR algorithm processes 8 bytes at once using clever bitwise tricks.
//
// Strategy:
//   1. Ensure all bytes are ASCII (high bit clear)
//      - ASCII bytes have values 0-127 (bit 7 = 0)
//      - Non-ASCII bytes have values 128-255 (bit 7 = 1)
//      - We need this check because our arithmetic tricks (XOR, subtract) assume
//        ASCII range. Non-ASCII bytes could produce false positives in later steps.
//      - The `is_ascii` mask will be used to filter out any non-ASCII results.
//   2. Detect bytes < 32 (control characters)
//   3. Detect bytes == 34 (quote character)
//   4. Detect bytes == 92 (backslash character)
//   5. Combine all results with OR

/// Check if any byte in 8 bytes needs JSON escaping (SWAR version).
///
/// Processes 8 bytes packed in a u64 simultaneously.
///
/// # Example
/// ```
/// use scratchpad::json_escape_SWAR::has_json_escapable_byte_swar;
///
/// // Pack bytes: [10, 92, 34, 65, 66, 67, 68, 69]
/// let x = 0x4544434241225C0Au64;
/// assert!(has_json_escapable_byte_swar(x));
/// ```
#[inline]
pub fn has_json_escapable_byte_swar(x: u64) -> bool {
    // ───────────────────────────────────────────────────────────────
    // Step 1: Check that all bytes are ASCII (bit 7 is clear)
    // ───────────────────────────────────────────────────────────────
    //
    // is_ascii = 0x80808080_80808080 & ~x
    //
    // For each byte in x:
    //   - If byte has bit 7 set (non-ASCII), result byte is 0x00
    //   - If byte has bit 7 clear (ASCII), result byte is 0x80
    //
    // Example: x = 0x00_00_00_5A_22_5C_0A_00
    //
    //   ~x =                0xFF_FF_FF_A5_DD_A3_F5_FF
    //   0x80808080... =     0x80_80_80_80_80_80_80_80
    //   is_ascii =          0x80_80_80_80_80_80_80_80
    //                          ^   ^   ^   ^   ^   ^   ^   ^
    //                       All bytes are ASCII!

    let is_ascii = 0x8080808080808080u64 & !x;

    // ───────────────────────────────────────────────────────────────
    // Step 2: Detect bytes < 32 (control characters)
    // ───────────────────────────────────────────────────────────────
    //
    // Goal: Detect if any byte is less than 32 (0-31 range)
    //
    // Subtract 0x20 (32) from each byte:
    //   - If byte < 32: result underflows (wraps around), setting bit 7 to 1
    //   - If byte >= 32: result is non-negative, bit 7 stays 0
    //
    // Why subtract 32? Because any value less than 32 will underflow:
    //   - Byte 0:  0 - 32 = -32 = 0xE0 (underflow! bit 7 = 1)
    //   - Byte 31: 31 - 32 = -1 = 0xFF (underflow! bit 7 = 1)
    //   - Byte 32: 32 - 32 = 0 = 0x00 (no underflow, bit 7 = 0)
    //   - Byte 65: 65 - 32 = 33 = 0x21 (no underflow, bit 7 = 0)
    //
    // Example: x = 0x00_41_22_0A_00_00_00_00
    //                   'A' "  \n
    //
    //   lt32 = x - 0x20... = 0xE0_21_02_EA_E0_E0_E0_E0
    //          - Byte 0:  0 - 32 = 0xE0 (underflow! bit 7 = 1)
    //          - Byte 10: 10 - 32 = 0xEA (underflow! bit 7 = 1)
    //          - Byte 34: 34 - 32 = 0x02 (no underflow, bit 7 = 0)
    //          - Byte 65: 65 - 32 = 0x21 (no underflow, bit 7 = 0)

    let lt32 = x.wrapping_sub(0x2020202020202020u64);

    // ───────────────────────────────────────────────────────────────
    // Step 3: Detect bytes == 34 (quote character)
    // ───────────────────────────────────────────────────────────────
    //
    // Goal: Detect if any byte equals 34 (quote: ")
    //
    // XOR with 0x22 (34) zeros out any byte that equals 34:
    //   - Byte == 34: 34 ^ 34 = 0
    //   - Byte != 34: non-zero result
    //
    // Then subtract 0x01 from each byte:
    //   - If byte was 0 (==34): 0 - 1 = 0xFF (underflow, bit 7 set)
    //   - If byte was != 0: result varies, but won't consistently set bit 7
    //
    // Example: x = 0x00_41_22_0A_00_00_00_00
    //                   'A' "  \n
    //
    //   sub34 = x ^ 0x22... = 0x22_63_00_28_22_22_22_22
    //          - Byte 34: 34 ^ 34 = 0x00 (zeroed!)
    //          - Others: non-zero values
    //
    //   eq34 = sub34 - 0x01... = 0x21_62_FF_27_21_21_21_21
    //          - Byte that was 0: 0 - 1 = 0xFF (underflow! bit 7 = 1)
    //          - Others: various values, no consistent bit 7

    let sub34 = x ^ 0x2222222222222222u64;
    let eq34 = sub34.wrapping_sub(0x0101010101010101u64);

    // ───────────────────────────────────────────────────────────────
    // Step 4: Detect bytes == 92 (backslash character)
    // ───────────────────────────────────────────────────────────────
    //
    // Goal: Detect if any byte equals 92 (backslash: \)
    //
    // XOR with 0x5C (92) zeros out any byte that equals 92:
    //   - Byte == 92: 92 ^ 92 = 0
    //   - Byte != 92: non-zero result
    //
    // Then subtract 0x01 from each byte:
    //   - If byte was 0 (==92): 0 - 1 = 0xFF (underflow, bit 7 set)
    //   - If byte was != 0: result varies, but won't consistently set bit 7
    //
    // Example: x = 0x00_41_5C_0A_00_00_00_00
    //                   'A' \  \n
    //
    //   sub92 = x ^ 0x5C... = 0x5C_1D_00_56_5C_5C_5C_5C
    //          - Byte 92: 92 ^ 92 = 0x00 (zeroed!)
    //          - Others: non-zero values
    //
    //   eq92 = sub92 - 0x01... = 0x5B_1C_FF_55_5B_5B_5B_5B
    //          - Byte that was 0: 0 - 1 = 0xFF (underflow! bit 7 = 1)
    //          - Others: various values, no consistent bit 7

    let sub92 = x ^ 0x5C5C5C5C5C5C5C5Cu64;
    let eq92 = sub92.wrapping_sub(0x0101010101010101u64);

    // ───────────────────────────────────────────────────────────────
    // Step 5: Combine all checks
    // ───────────────────────────────────────────────────────────────
    //
    // (lt32 | eq34 | eq92) & is_ascii
    //
    // - lt32 has bit 7 set for bytes < 32
    // - eq34 has bit 7 set for bytes == 34
    // - eq92 has bit 7 set for bytes == 92
    // - OR them all together to get bytes that match any condition
    // - AND with is_ascii to ensure we only flag ASCII bytes
    //
    // Result != 0 means at least one byte needs escaping!

    ((lt32 | eq34 | eq92) & is_ascii) != 0
}

/// Check if any byte in a buffer needs JSON escaping (SWAR version).
///
/// Processes the buffer in 8-byte chunks using SWAR for efficiency.
pub fn has_json_escapable_byte(buffer: &[u8]) -> bool {
    let mut i = 0;

    // Process 8 bytes at a time
    while i + 8 <= buffer.len() {
        let chunk = u64::from_le_bytes([
            buffer[i],
            buffer[i + 1],
            buffer[i + 2],
            buffer[i + 3],
            buffer[i + 4],
            buffer[i + 5],
            buffer[i + 6],
            buffer[i + 7],
        ]);

        if has_json_escapable_byte_swar(chunk) {
            return true;
        }

        i += 8;
    }

    // Handle remaining bytes (< 8) with scalar
    buffer[i..].iter().any(|&b| needs_json_escape_scalar(b))
}

// ═══════════════════════════════════════════════════════════════════════════
//                    Helper: Find Position of Escapable Byte
// ═══════════════════════════════════════════════════════════════════════════

/// Find the index of the first byte that needs JSON escaping.
///
/// Returns None if no byte needs escaping.
pub fn find_first_escapable(buffer: &[u8]) -> Option<usize> {
    for (i, &byte) in buffer.iter().enumerate() {
        if needs_json_escape_scalar(byte) {
            return Some(i);
        }
    }
    None
}

// ═══════════════════════════════════════════════════════════════════════════
//                                 Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_control_chars() {
        assert!(needs_json_escape_scalar(0));   // NULL
        assert!(needs_json_escape_scalar(10));  // LF
        assert!(needs_json_escape_scalar(13));  // CR
        assert!(needs_json_escape_scalar(31));  // Unit separator
    }

    #[test]
    fn test_scalar_quote() {
        assert!(needs_json_escape_scalar(34));  // "
    }

    #[test]
    fn test_scalar_backslash() {
        assert!(needs_json_escape_scalar(92));  // \
    }

    #[test]
    fn test_scalar_normal_chars() {
        assert!(!needs_json_escape_scalar(32));  // Space
        assert!(!needs_json_escape_scalar(65));  // A
        assert!(!needs_json_escape_scalar(97));  // a
        assert!(!needs_json_escape_scalar(126)); // ~
    }

    #[test]
    fn test_swar_clean_bytes() {
        // "Hello!!!" - all clean ASCII characters
        let x = u64::from_le_bytes([b'H', b'e', b'l', b'l', b'o', b'!', b'!', b'!']);
        assert!(!has_json_escapable_byte_swar(x));
    }

    #[test]
    fn test_swar_with_quote() {
        // Contains quote character (34)
        let x = u64::from_le_bytes([b'H', b'"', b'i', b' ', b' ', b' ', b' ', b' ']);
        assert!(has_json_escapable_byte_swar(x));
    }

    #[test]
    fn test_swar_with_backslash() {
        // Contains backslash (92)
        let x = u64::from_le_bytes([b'A', b'\\', b'B', b' ', b' ', b' ', b' ', b' ']);
        assert!(has_json_escapable_byte_swar(x));
    }

    #[test]
    fn test_swar_with_control() {
        // Contains newline (10)
        let x = u64::from_le_bytes([b'A', b'\n', b'B', b' ', b' ', b' ', b' ', b' ']);
        assert!(has_json_escapable_byte_swar(x));
    }

    #[test]
    fn test_swar_with_tab() {
        // Contains tab (9)
        let x = u64::from_le_bytes([b'A', b'\t', b'B', b' ', b' ', b' ', b' ', b' ']);
        assert!(has_json_escapable_byte_swar(x));
    }

    #[test]
    fn test_buffer_clean() {
        let buffer = b"Hello, World!";
        assert!(!has_json_escapable_byte(buffer));
        assert!(!has_json_escapable_byte_scalar(buffer));
    }

    #[test]
    fn test_buffer_with_quote() {
        let buffer = b"Hello \"World\"!";
        assert!(has_json_escapable_byte(buffer));
        assert!(has_json_escapable_byte_scalar(buffer));
        assert_eq!(find_first_escapable(buffer), Some(6));
    }

    #[test]
    fn test_buffer_with_backslash() {
        let buffer = b"path\\to\\file";
        assert!(has_json_escapable_byte(buffer));
        assert!(has_json_escapable_byte_scalar(buffer));
        assert_eq!(find_first_escapable(buffer), Some(4));
    }

    #[test]
    fn test_buffer_with_newline() {
        let buffer = b"Line 1\nLine 2";
        assert!(has_json_escapable_byte(buffer));
        assert!(has_json_escapable_byte_scalar(buffer));
        assert_eq!(find_first_escapable(buffer), Some(6));
    }

    #[test]
    fn test_buffer_with_tab() {
        let buffer = b"Col1\tCol2\tCol3";
        assert!(has_json_escapable_byte(buffer));
        assert!(has_json_escapable_byte_scalar(buffer));
        assert_eq!(find_first_escapable(buffer), Some(4));
    }

    #[test]
    fn test_buffer_various_lengths() {
        // Test buffers of different lengths to ensure proper handling
        // of both 8-byte chunks and remainder bytes

        // Exactly 8 bytes, no escapable
        let buf8 = b"12345678";
        assert!(!has_json_escapable_byte(buf8));

        // Exactly 8 bytes, with escapable at end
        let buf8_esc = b"1234567\"";
        assert!(has_json_escapable_byte(buf8_esc));

        // 16 bytes, escapable in first chunk
        let buf16_first = b"12\"4567890123456";
        assert!(has_json_escapable_byte(buf16_first));

        // 16 bytes, escapable in second chunk
        let buf16_second = b"12345678901234\"6";
        assert!(has_json_escapable_byte(buf16_second));

        // 13 bytes, escapable in remainder
        let buf13_remainder = b"123456789012\"";
        assert!(has_json_escapable_byte(buf13_remainder));

        // 13 bytes, no escapable
        let buf13_clean = b"1234567890123";
        assert!(!has_json_escapable_byte(buf13_clean));
    }

    #[test]
    fn test_swar_matches_scalar() {
        // Test that SWAR and scalar produce same results
        let test_cases = vec![
            b"" as &[u8],
            b"Hello",
            b"Hello \"World\"",
            b"Path\\to\\file",
            b"Line1\nLine2\nLine3",
            b"Tab\tseparated\tvalues",
            b"\x00\x01\x02\x03\x04",  // Control characters
            b"Mixed \"quotes\" and \\backslashes\\ and \nnewlines",
        ];

        for test in test_cases {
            let swar_result = has_json_escapable_byte(test);
            let scalar_result = has_json_escapable_byte_scalar(test);
            assert_eq!(
                swar_result, scalar_result,
                "Mismatch for input: {:?}",
                std::str::from_utf8(test).unwrap_or("<invalid utf8>")
            );
        }
    }

    #[test]
    fn test_edge_cases() {
        // Byte 32 (space) should NOT need escaping
        assert!(!needs_json_escape_scalar(32));
        let x = u64::from_le_bytes([32, 32, 32, 32, 32, 32, 32, 32]);
        assert!(!has_json_escapable_byte_swar(x));

        // Byte 33 should NOT need escaping
        assert!(!needs_json_escape_scalar(33));
        let x = u64::from_le_bytes([33, 33, 33, 33, 33, 33, 33, 33]);
        assert!(!has_json_escapable_byte_swar(x));

        // Byte 91 (just before backslash) should NOT need escaping
        assert!(!needs_json_escape_scalar(91));
        let x = u64::from_le_bytes([91, 91, 91, 91, 91, 91, 91, 91]);
        assert!(!has_json_escapable_byte_swar(x));

        // Byte 93 (just after backslash) should NOT need escaping
        assert!(!needs_json_escape_scalar(93));
        let x = u64::from_le_bytes([93, 93, 93, 93, 93, 93, 93, 93]);
        assert!(!has_json_escapable_byte_swar(x));
    }
}
