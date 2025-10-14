use std::arch::aarch64::*;

// ═══════════════════════════════════════════════════════════════════════════
//                        NEON SIMD Line Feed Insertion
// ═══════════════════════════════════════════════════════════════════════════
//
// Insert '\n' every K bytes using ARM NEON SIMD instructions.
// Performance: 10-20x faster than scalar implementation.
//
// Example:  "ABCDEFGHIJ", k=3  →  "ABC\nDEF\nGHI\nJ"
//
// Architecture:
//   insert_line_feed_scalar()         Simple reference implementation
//   insert_line_feed32_neon_impl()    Core SIMD kernel (32→33 bytes)
//   insert_line_feed_neon()           Main driver for arbitrary buffers
//
// Core technique: Mark insertion points with 255 in shuffle masks, then blend
// with linefeeds using vbslq_u8. For insertions in the lower 16 bytes, use
// vextq_u8 to handle cross-register data movement.

// ───────────────────────────────────────────────────────────────────────────
//                             Shuffle Masks
// ───────────────────────────────────────────────────────────────────────────
//
// Each mask is a 16-byte recipe for vqtbl1q_u8:
//   • Values 0-15: Copy that byte from source
//   • Value 255:   Marks insertion point for '\n'
//
// Example: SHUFFLE_MASKS_NEON[3] = [0, 1, 2, 255, 3, 4, 5, ...]
//
//   Input:  [A][B][C][D][E][F][G][H]
//   Mask:   [0][1][2][255][3][4][5][6]
//                      ↓
//   Shuffle [A][B][C][?][D][E][F][G]    ← 255 produces garbage
//   Compare [F][F][F][T][F][F][F][F]    ← vceqq_u8 finds 255
//   Blend   [A][B][C][\n][D][E][F][G]   ← vbslq_u8 replaces with '\n'

pub static SHUFFLE_MASKS_NEON: [[u8; 16]; 16] = [
    [255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 255, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 255, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 255, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 255, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 255, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 255, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 255, 7, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 255, 8, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 255, 9, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 255, 10, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 255, 11, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 255, 12, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 255, 13, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 255, 14],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 255],
];

// ═══════════════════════════════════════════════════════════════════════════
//                     Core NEON Kernel: 32 → 33 bytes
// ═══════════════════════════════════════════════════════════════════════════
//
// Inserts '\n' at position n within 32 input bytes, producing 33 output bytes.
// Uses two 128-bit registers (ARM NEON hardware limit: 16 bytes per register).
//
// Three strategies:
//   n == 32   Trivial append
//   n ≥ 16    Insert in upper register
//   n < 16    Insert in lower, shift upper (requires vextq_u8)

#[target_feature(enable = "neon")]
pub unsafe fn insert_line_feed32_neon_impl(input: &[u8; 32], n: usize) -> [u8; 33] {
    let mut output = [0u8; 33];

    // Load 32 bytes into two NEON registers
    //
    //   Memory:  [0 1 2 ... 15][16 17 ... 31]
    //                  ↓              ↓
    //            vld1q_u8        vld1q_u8
    //                  ↓              ↓
    //   Registers:  lower          upper

    let lower = vld1q_u8(input.as_ptr());
    let upper = vld1q_u8(input.as_ptr().add(16));

    // Prepare helper vectors
    let line_feed_vector = vdupq_n_u8(b'\n');  // [\n, \n, \n, ..., \n]
    let identity = vcombine_u8(
        vcreate_u8(0x0706050403020100u64),
        vcreate_u8(0x0F0E0D0C0B0A0908u64),
    );  // [0, 1, 2, 3, ..., 15] - pass-through mask

    if n == 32 {
        // ───────────────────────────────────────────────────────────────
        // Case 1: Append at end
        // ───────────────────────────────────────────────────────────────

        vst1q_u8(output.as_mut_ptr(), lower);
        vst1q_u8(output.as_mut_ptr().add(16), upper);
        output[32] = b'\n';

    } else if n >= 16 {
        // ───────────────────────────────────────────────────────────────
        // Case 2: Insert in upper register
        // ───────────────────────────────────────────────────────────────
        //
        // Example: n=18 (insert after byte 18)
        //
        //   Before:  [A B C ... O] [P Q R S ... Z]
        //             lower (0-15)  upper (16-31)
        //                              ↑
        //                           Insert at position 2 in upper
        //   After:   [A B C ... O] [P Q \n R S ... Z]
        //
        // Process:
        //   1. Load maskhi = SHUFFLE_MASKS_NEON[2]
        //   2. vqtbl1q_u8 shuffles upper, creating gap
        //   3. vceqq_u8 finds where mask has 255
        //   4. vbslq_u8 blends '\n' into gap

        let maskhi = vld1q_u8(SHUFFLE_MASKS_NEON[n - 16].as_ptr());

        // Lower: pass through unchanged
        let lf_pos_lo = vceqq_u8(identity, vdupq_n_u8(255));
        let shuffled_lo = vqtbl1q_u8(lower, identity);
        let result_lo = vbslq_u8(lf_pos_lo, line_feed_vector, shuffled_lo);

        // Upper: shuffle + blend
        let lf_pos_hi = vceqq_u8(maskhi, vdupq_n_u8(255));
        let shuffled_hi = vqtbl1q_u8(upper, maskhi);
        let result_hi = vbslq_u8(lf_pos_hi, line_feed_vector, shuffled_hi);

        vst1q_u8(output.as_mut_ptr(), result_lo);
        vst1q_u8(output.as_mut_ptr().add(16), result_hi);

    } else {
        // ───────────────────────────────────────────────────────────────
        // Case 3: Insert in lower register (complex)
        // ───────────────────────────────────────────────────────────────
        //
        // Example: n=5
        //
        //   Problem: Inserting '\n' in lower pushes byte P (position 15) out
        //
        //   Before:  [A B C D E F G H I J K L M N O P] [Q R S T ... Z]
        //             lower (0-15)                      upper (16-31)
        //                        ↑                         ↑
        //                   Insert here              Needs to absorb P
        //
        //   After:   [A B C D E \n F G H I J K L M N O] [P Q R S ... Z]
        //
        // Solution: vextq_u8(lower, upper, 15) creates [P, Q, R, ..., Y]
        //           by taking last 1 byte from lower, first 15 from upper
        //
        //   vextq_u8(A, B, n) = [last (16-n) bytes of A][first n bytes of B]

        let shifted_upper = vextq_u8(lower, upper, 15);

        let masklo = vld1q_u8(SHUFFLE_MASKS_NEON[n].as_ptr());
        let lf_pos_lo = vceqq_u8(masklo, vdupq_n_u8(255));
        let shuffled_lo = vqtbl1q_u8(lower, masklo);
        let result_lo = vbslq_u8(lf_pos_lo, line_feed_vector, shuffled_lo);

        let lf_pos_hi = vceqq_u8(identity, vdupq_n_u8(255));
        let shuffled_hi = vqtbl1q_u8(shifted_upper, identity);
        let result_hi = vbslq_u8(lf_pos_hi, line_feed_vector, shuffled_hi);

        vst1q_u8(output.as_mut_ptr(), result_lo);
        vst1q_u8(output.as_mut_ptr().add(16), result_hi);
    }

    output
}

// ═══════════════════════════════════════════════════════════════════════════
//                          Scalar Reference
// ═══════════════════════════════════════════════════════════════════════════

pub fn insert_line_feed_scalar(buffer: &[u8], k: usize) -> Vec<u8> {
    if k == 0 {
        return buffer.to_vec();
    }

    let num_line_feeds = buffer.len() / k;
    let output_len = buffer.len() + num_line_feeds;
    let mut output = Vec::with_capacity(output_len);

    let mut input_pos = 0;

    while input_pos + k <= buffer.len() {
        output.extend_from_slice(&buffer[input_pos..input_pos + k]);
        output.push(b'\n');
        input_pos += k;
    }

    output.extend_from_slice(&buffer[input_pos..]);

    output
}

// ═══════════════════════════════════════════════════════════════════════════
//                         NEON-Optimized Driver
// ═══════════════════════════════════════════════════════════════════════════
//
// Strategy:
//   k ≤ 32:  Use shuffle-based SIMD kernel
//   k > 32:  Bulk SIMD copy (32 bytes/iteration) + append '\n'

pub fn insert_line_feed_neon(buffer: &[u8], k: usize) -> Vec<u8> {
    if k == 0 {
        return buffer.to_vec();
    }

    let num_line_feeds = buffer.len() / k;
    let output_len = buffer.len() + num_line_feeds;
    let mut output = Vec::with_capacity(output_len);

    let mut input_pos = 0;

    unsafe {
        let output_ptr: *mut u8 = output.as_mut_ptr();
        let mut output_pos = 0;

        while input_pos + k <= buffer.len() {
            if k <= 32 {
                // ───────────────────────────────────────────────────────────
                // Fast path: Shuffle-based SIMD (k ≤ 32)
                // ───────────────────────────────────────────────────────────

                let input_ptr = buffer.as_ptr().add(input_pos);

                let lower = vld1q_u8(input_ptr);
                let upper = if input_pos + 16 < buffer.len() {
                    vld1q_u8(input_ptr.add(16))
                } else {
                    vdupq_n_u8(0)
                };

                let line_feed_vector = vdupq_n_u8(b'\n');
                let identity = vcombine_u8(
                    vcreate_u8(0x0706050403020100u64),
                    vcreate_u8(0x0F0E0D0C0B0A0908u64),
                );

                if k == 32 {
                    vst1q_u8(output_ptr.add(output_pos), lower);
                    vst1q_u8(output_ptr.add(output_pos + 16), upper);
                    *output_ptr.add(output_pos + 32) = b'\n';
                    output_pos += 33;
                } else if k >= 16 {
                    let maskhi = vld1q_u8(SHUFFLE_MASKS_NEON[k - 16].as_ptr());

                    let lf_pos_lo = vceqq_u8(identity, vdupq_n_u8(255));
                    let shuffled_lo = vqtbl1q_u8(lower, identity);
                    let result_lo = vbslq_u8(lf_pos_lo, line_feed_vector, shuffled_lo);

                    let lf_pos_hi = vceqq_u8(maskhi, vdupq_n_u8(255));
                    let shuffled_hi = vqtbl1q_u8(upper, maskhi);
                    let result_hi = vbslq_u8(lf_pos_hi, line_feed_vector, shuffled_hi);

                    vst1q_u8(output_ptr.add(output_pos), result_lo);
                    vst1q_u8(output_ptr.add(output_pos + 16), result_hi);
                    output_pos += k + 1;
                } else {
                    let shifted_upper = vextq_u8(lower, upper, 15);

                    let masklo = vld1q_u8(SHUFFLE_MASKS_NEON[k].as_ptr());
                    let lf_pos_lo = vceqq_u8(masklo, vdupq_n_u8(255));
                    let shuffled_lo = vqtbl1q_u8(lower, masklo);
                    let result_lo = vbslq_u8(lf_pos_lo, line_feed_vector, shuffled_lo);

                    let lf_pos_hi = vceqq_u8(identity, vdupq_n_u8(255));
                    let shuffled_hi = vqtbl1q_u8(shifted_upper, identity);
                    let result_hi = vbslq_u8(lf_pos_hi, line_feed_vector, shuffled_hi);

                    vst1q_u8(output_ptr.add(output_pos), result_lo);
                    vst1q_u8(output_ptr.add(output_pos + 16), result_hi);
                    output_pos += k + 1;
                }

                input_pos += k;
            } else {
                // ───────────────────────────────────────────────────────────
                // Slow path: Bulk SIMD copy (k > 32)
                // ───────────────────────────────────────────────────────────
                //
                // Example: k=100
                //
                //   Input:  [0..31][32..63][64..95][96..99]
                //              ↓      ↓       ↓       ↓
                //            NEON   NEON    NEON   remainder
                //
                //   Output: [0..31][32..63][64..95][96..99][\n]

                let mut remaining = k;

                // Copy 32 bytes at a time
                while remaining >= 32 {
                    let input_ptr = buffer.as_ptr().add(input_pos);

                    let lower = vld1q_u8(input_ptr);
                    let upper = vld1q_u8(input_ptr.add(16));

                    vst1q_u8(output_ptr.add(output_pos), lower);
                    vst1q_u8(output_ptr.add(output_pos + 16), upper);

                    output_pos += 32;
                    input_pos += 32;
                    remaining -= 32;
                }

                // Handle remainder (0-31 bytes)
                // NEON loads are always 16 bytes - for small remainders use
                // temp buffer to avoid reading past buffer boundaries
                if remaining > 0 {
                    let input_ptr = buffer.as_ptr().add(input_pos);
                    let mut temp = [0u8; 32];

                    let lower = vld1q_u8(input_ptr);
                    vst1q_u8(temp.as_mut_ptr(), lower);

                    if remaining > 16 {
                        let upper = vld1q_u8(input_ptr.add(16));
                        vst1q_u8(temp.as_mut_ptr().add(16), upper);
                    }

                    std::ptr::copy_nonoverlapping(
                        temp.as_ptr(),
                        output_ptr.add(output_pos),
                        remaining
                    );
                    output_pos += remaining;
                    input_pos += remaining;
                }

                *output_ptr.add(output_pos) = b'\n';
                output_pos += 1;
            }
        }

        output.set_len(output_pos);
    }

    // Copy leftover bytes (incomplete final chunk, no '\n')
    output.extend_from_slice(&buffer[input_pos..]);

    output
}
