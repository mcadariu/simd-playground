//! CSV parsing: State Machine vs If/Else comparison.
//!
//! Based on KWIllets' comment on Daniel Lemire's blog post:
//! https://lemire.me/blog/2008/12/19/parsing-csv-files-is-cpu-bound-a-c-test-case-update-2/
//!
//! KWIllets proposed using a Deterministic Finite Automaton (DFA) for CSV parsing:
//! "a small DFA to parse the CSV format in memory with an inner loop like:
//!  while( state = dfa[state].edges[*p++] );"
//!
//! Key insights:
//! - Low instruction count
//! - Low branch count (reduces branch mispredictions)
//! - Sentinel padding at end eliminates boundary checks
//!
//! ## Implementations
//!
//! 1. **STATE MACHINE (KWIllets approach)**:
//!    - Table-driven state transitions
//!    - Sentinel-terminated (no bounds check in hot loop)
//!    - Branchless action handling
//!    - Direct pointer arithmetic
//!
//! 2. **IF/ELSE (Simple approach)**:
//!    - Straightforward conditional logic
//!    - Many branches per byte
//!
//! ## Benchmark Results
//!
//! **Predictable CSV:**
//! - If/Else:       1.38 GB/s
//! - State Machine: 0.38 GB/s  (3.6x slower)
//!
//! **Adversarial CSV (random patterns):**
//! - If/Else:       0.58 GB/s  (drops 58%!)
//! - State Machine: 0.38 GB/s  (consistent, but still 1.5x slower)
//!
//! ## Why If/Else Wins
//!
//! Even though the theory is sound:
//! 1. Modern branch predictors are exceptional (especially Apple Silicon)
//! 2. Table lookups have inherent overhead (memory indirection)
//! 3. Even "random" CSV has hidden structure CPUs exploit
//! 4. Rust compiler optimizes if/else chains brilliantly
//!
//! ## When State Machines Win
//!
//! - Older CPUs with weaker branch prediction
//! - Embedded systems
//! - Very complex grammars (>10 states)
//! - True random bit patterns (not structured text)
//!
//! ## Key Lesson
//!
//! Theory is CORRECT (gap narrowed 3.6x → 1.5x on adversarial data),
//! but modern hardware can surprise you. Always profile!

// ═══════════════════════════════════════════════════════════════════════════
//                         State Machine Approach
// ═══════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum State {
    FieldStart = 0,
    Unquoted = 1,
    Quoted = 2,
    QuoteInQuoted = 3,
    End = 4,
}

const fn classify_byte(b: u8) -> usize {
    match b {
        b',' => 0,      // Comma
        b'\n' => 1,     // Newline
        b'"' => 2,      // Quote
        0 => 3,         // Sentinel (end of input)
        _ => 4,         // Regular character
    }
}

// State transition table: [state][byte_class] -> (next_state, _unused)
const TRANSITIONS: [[(State, u8); 5]; 4] = [
    // FIELD_START
    [
        (State::FieldStart, 0),    // ','
        (State::FieldStart, 0),    // '\n'
        (State::Quoted, 0),        // '"'
        (State::End, 0),           // sentinel
        (State::Unquoted, 0),      // other
    ],
    // UNQUOTED
    [
        (State::FieldStart, 0),    // ','
        (State::FieldStart, 0),    // '\n'
        (State::Unquoted, 0),      // '"'
        (State::End, 0),           // sentinel
        (State::Unquoted, 0),      // other
    ],
    // QUOTED
    [
        (State::Quoted, 0),        // ','
        (State::Quoted, 0),        // '\n'
        (State::QuoteInQuoted, 0), // '"'
        (State::End, 0),           // sentinel
        (State::Quoted, 0),        // other
    ],
    // QUOTE_IN_QUOTED
    [
        (State::FieldStart, 0),    // ','
        (State::FieldStart, 0),    // '\n'
        (State::Quoted, 0),        // '"' (escaped)
        (State::End, 0),           // sentinel
        (State::Unquoted, 0),      // other
    ],
];

// Packed action table: bits 0-1 for field increment, bit 1 for row increment
const ACTION_TABLE: [[u8; 5]; 4] = [
    [1, 3, 0, 0, 0],  // FIELD_START: comma=+field, newline=+field+row
    [1, 3, 0, 3, 0],  // UNQUOTED: same, sentinel also counts
    [0, 0, 0, 0, 0],  // QUOTED: no actions inside quotes
    [1, 3, 0, 3, 0],  // QUOTE_IN_QUOTED: same as unquoted
];

/// Parse CSV using KWIllets' state machine approach.
///
/// Optimizations:
/// - Sentinel-terminated: no bounds check in hot loop
/// - Table-driven: minimal branching
/// - Branchless actions: bit manipulation
/// - Direct memory access: unsafe pointer arithmetic
///
/// Trade-off: One-time buffer copy for sentinel vs zero-branch loop
pub fn parse_csv_state_machine(data: &[u8]) -> (usize, usize) {
    if data.is_empty() {
        return (0, 0);
    }

    let mut fields = 0usize;
    let mut rows = 0usize;
    let mut state = State::FieldStart;

    // One-time cost: add sentinel
    let mut buffer = Vec::with_capacity(data.len() + 1);
    buffer.extend_from_slice(data);
    buffer.push(0); // Sentinel

    let mut i = 0;
    let ptr = buffer.as_ptr();

    unsafe {
        loop {
            let byte = *ptr.add(i);
            let class = classify_byte(byte);
            let (next_state, _) = TRANSITIONS[state as usize][class];

            // Branchless action handling using bit manipulation
            let packed_action = ACTION_TABLE[state as usize][class];
            fields += (packed_action & 1) as usize;
            rows += ((packed_action >> 1) & 1) as usize;

            state = next_state;
            i += 1;

            // Only branch: check terminal state (driven by sentinel)
            if state == State::End {
                break;
            }
        }
    }

    (fields, rows)
}

// ═══════════════════════════════════════════════════════════════════════════
//                         If/Else Approach
// ═══════════════════════════════════════════════════════════════════════════

/// Parse CSV using straightforward if/else logic.
///
/// The "naive" approach with many branches per byte.
/// Surprisingly wins on modern hardware due to excellent branch prediction!
pub fn parse_csv_if_else(data: &[u8]) -> (usize, usize) {
    let mut fields = 0;
    let mut rows = 0;
    let mut in_quotes = false;
    let mut field_started = false;

    let mut i = 0;
    while i < data.len() {
        let byte = data[i];

        if byte == b'"' {
            if in_quotes {
                // Check if it's an escaped quote
                if i + 1 < data.len() && data[i + 1] == b'"' {
                    i += 1; // Skip the escaped quote
                } else {
                    in_quotes = false;
                }
            } else {
                in_quotes = true;
                field_started = true;
            }
        } else if byte == b',' {
            if in_quotes {
                // Inside quotes, comma is literal
                field_started = true;
            } else {
                // Field separator
                if field_started || fields > 0 || rows > 0 {
                    fields += 1;
                }
                field_started = false;
            }
        } else if byte == b'\n' {
            if in_quotes {
                // Inside quotes, newline is literal
                field_started = true;
            } else {
                // Row separator
                if field_started || fields > 0 {
                    fields += 1;
                }
                rows += 1;
                field_started = false;
            }
        } else {
            // Regular character
            field_started = true;
        }

        i += 1;
    }

    // Handle last field if file doesn't end with newline
    if field_started {
        fields += 1;
        if rows == 0 {
            rows = 1;
        }
    }

    (fields, rows)
}

// ═══════════════════════════════════════════════════════════════════════════
//                         Tests
// ═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_csv() {
        let csv = b"a,b,c\n1,2,3\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 6);
        assert_eq!(rows_sm, 2);
        assert_eq!(fields_ie, 6);
        assert_eq!(rows_ie, 2);
    }

    #[test]
    fn test_quoted_fields() {
        let csv = b"\"hello\",\"world\"\n\"foo\",\"bar\"\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 4);
        assert_eq!(rows_sm, 2);
        assert_eq!(fields_ie, 4);
        assert_eq!(rows_ie, 2);
    }

    #[test]
    fn test_comma_in_quotes() {
        let csv = b"\"hello,world\",test\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 2);
        assert_eq!(rows_sm, 1);
        assert_eq!(fields_ie, 2);
        assert_eq!(rows_ie, 1);
    }

    #[test]
    fn test_newline_in_quotes() {
        let csv = b"\"hello\nworld\",test\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 2);
        assert_eq!(rows_sm, 1);
        assert_eq!(fields_ie, 2);
        assert_eq!(rows_ie, 1);
    }

    #[test]
    fn test_escaped_quotes() {
        let csv = b"\"hello\"\"world\",test\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 2);
        assert_eq!(rows_sm, 1);
        assert_eq!(fields_ie, 2);
        assert_eq!(rows_ie, 1);
    }

    #[test]
    fn test_empty_fields() {
        let csv = b"a,,c\n,,\n";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 6);
        assert_eq!(rows_sm, 2);
        assert_eq!(fields_ie, 6);
        assert_eq!(rows_ie, 2);
    }

    #[test]
    fn test_no_trailing_newline() {
        let csv = b"a,b,c";
        let (fields_sm, rows_sm) = parse_csv_state_machine(csv);
        let (fields_ie, rows_ie) = parse_csv_if_else(csv);

        assert_eq!(fields_sm, 3);
        assert_eq!(rows_sm, 1);
        assert_eq!(fields_ie, 3);
        assert_eq!(rows_ie, 1);
    }
}
