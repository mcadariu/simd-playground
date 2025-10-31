[Daniel Lemire's](https://lemire.me/blog/) and [Wojciech Muła's](http://0x80.pl/) blogs are great resources full of common problems and (very) efficient solutions (most often SIMD-based) encountered in the data processing domain. 
As I was reading the posts, I have observed recurring themes. The goal is to reproduce them on my laptop and gather them in this repo as a quick index of "recipes". 

# Convert

* [ASCII to lower case (AVX)](https://lemire.me/blog/2024/08/03/converting-ascii-strings-to-lower-case-at-crazy-speeds-with-avx-512)
  * load 'A' and 'Z' into regs
  * create mask with elements already uppercase (compare input to A and Z, then do an AND with both)
  * add difference between 'A' and 'a' to elements according to mask
* [Packing a string of digits into an integer](https://lemire.me/blog/2023/07/07/packing-a-string-of-digits-into-an-integer-quickly)
  * leverage fact that '2' is 0x32
  * mask to get the high nibbles, 0x32 -> 2
  * shuffle with table that reorders and eliminates ' '; you then have 0x00  0x01  0x09  0x02...
  * shift to create empty space (zeros) where the other digit will go, then OR fills that space without destroying the original digit.
  * narrow from 16-bit to 8-bit
  * extract 64 bit integer
* [Trimming spaces from strings (SVE)](https://lemire.me/blog/2023/03/10/trimming-spaces-from-strings-faster-with-sve-on-an-amazon-graviton-3-processor/)
  * Get vector width in 32-bit elements
  * Load bytes, extend to 32-bit
  * Compare not-equal, create predicate mask
  * Pack elements where mask=1 (removes gaps using compact)
  * Store low bytes
  * Count 1-bits in predicate mask
  * Create mask for range [start, end) to process leftover
* [Removing chars from strings (AVX)](https://lemire.me/blog/2022/04/28/removing-characters-from-strings-faster-with-avx-512)
  * detect where whitespace is, create a mask, compress, popcnt to know how much to advance 
* [Integers to decimal strings (AVX)](https://lemire.me/blog/2022/03/28/converting-integers-to-decimal-strings-faster-with-avx-512)
* [Integers to fix digits (SWAR)](https://lemire.me/blog/2021/11/18/converting-integers-to-fix-digit-representations-quickly/)
* [Removing duplicates](https://lemire.me/blog/2017/04/10/removing-duplicates-from-lists-quickly/)
  * work in batches of 8, “ov and nv”; we need a RECONSTRUCTION vector (last element from ov and first 7 from new) - you do it with BLEND (takes 2 vectors and a mask)
  * compare recon with nv, store in a mask
  * population count to know how many unique (to know how many we write to output)
  * uniqshuf[M] is a pre-computed lookup table that contains shuffle masks (SHUFFLE MASKS)
  * Example 3: `M = 0b00010100` (Bits 2 and 4 set) **Meaning**: Positions 2 and 4 are duplicates **Input** `nv`: `[10, 11, 11, 12, 12, 13, 14, 15]` - Position 2: `11` (duplicate) - Position 4: `12` (duplicate) **Shuffle mask** `uniqshuf[20]`: (20 =   		0b00010100) ``` [0, 1, 3, 5, 6, 7, 7, 7] // Skip indices 2 and 4 

### With insertion (intermediate step: slots)
* [Escaping strings (AVX)](https://lemire.me/blog/2022/09/14/escaping-strings-faster-with-avx-512)
  * get '\' and '"' into registers 
  * load and expand input with 0 every other byte to make space
  * create masks of where \ and " appear
  * create to_keep mask of above mask or 0xAAAAAAAA (010101...) - because we will shift 
  * shift (now \ can be placed before the char of interest) and blend with is quote or \ to get the escaped (still has 0's)
  * compress result to remove the 0 we don't need anymore (inputs: to_keep and escaped) 
  * advance the output pointer with with the number of written bytes (to know how many, do popcnt of to_keep)

### Decode / Transcode / Encode

* [Decoding Base16 sequences](https://lemire.me/blog/2023/07/27/decoding-base16-sequences-quickly)
  * load 16 chars
  * subtract 1 from each
  * extract high nibble (shift right 4, and with 0x0f
  * vectorised lookup (invalid will return "enough" to set MSB to 1, valid just small adjustment)
  * another lookup to get actual hex value
  * fused multiply-add
  * lastly, pack to obtain the result 
* [Latin 1 to UTF-8 (AVX)](https://lemire.me/blog/2023/08/18/transcoding-latin-1-strings-to-utf-8-strings-at-12-gb-s-using-avx-512)
* [UTF-8 to Latin 1 (AVX)](https://lemire.me/blog/2023/08/12/transcoding-utf-8-strings-to-latin-1-strings-at-12-gb-s-using-avx-512)
* [Base16 encoding](https://lemire.me/blog/2022/12/23/fast-base16-encoding)
  * extract high and low nibbles
  * interleave them from 4A and 3F to [0x04, 0x0A, 0x03, 0x0F..]
  * use lookup table to convert
* [Binary in ASCII (SWAR)](https://lemire.me/blog/2020/05/02/encoding-binary-in-ascii-very-fast)
* [Bitset decoding (AVX)](https://lemire.me/blog/2022/05/06/fast-bitset-decoding-using-intel-avx-512)
  * use compress (_mm512_mask_compressstoreu_epi32) using a shifting mask (e.g. at second iteration mask = (bits >> 16) & 0xFFFF) 

# Search

* [Control chars (SWAR)](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Identifiers (NEON)](https://lemire.me/blog/2023/09/04/locating-identifiers-quickly-arm-neon-edition)
* [String prefixes (SIMD)](https://lemire.me/blog/2023/07/14/recognizing-string-prefixes-with-simd-instructions/)
* [JSON escapable chars (SWAR)](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Escapable chars (SWAR)](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Identifying sequence of digits in string](https://lemire.me/blog/2018/09/30/quickly-identifying-a-sequence-of-digits-in-a-string-of-characters)

### Scan

* [HTML](https://lemire.me/blog/2024/07/05/scan-html-faster-with-simd-instructions-net-c-edition)
* [HTML](https://lemire.me/blog/2024/07/20/scan-html-even-faster-with-simd-instructions-c-and-c)
* [HTML](https://lemire.me/blog/2024/06/08/scan-html-faster-with-simd-instructions-chrome-edition)

# Parse 

### Numbers
* [Integers (AVX)](https://lemire.me/blog/2023/09/22/parsing-integers-quickly-with-avx-512)
* [Floating point](https://lemire.me/blog/2021/02/22/parsing-floating-point-numbers-really-fast-in-c)
* [String of digit into integer (SWAR)](https://lemire.me/blog/2022/01/21/swar-explained-parsing-eight-digits)

### Timestamps
* [Timestamps](https://lemire.me/blog/2023/07/01/parsing-time-stamps-faster-with-simd-instructions)

### IPs
* [IPs](https://lemire.me/blog/2023/06/08/parsing-ip-addresses-crazily-fast/)

# Filtering
* [Filtering numbers (SVE)](https://lemire.me/blog/2022/07/14/filtering-numbers-faster-with-sve-on-amazon-graviton-3-processors/)

# Prefix minimum

* [Prefix minimum](https://lemire.me/blog/2023/08/10/coding-of-domain-names-to-wire-format-at-gigabytes-per-second)

# Summarize 
* [Computing the UTF-8 size of a Latin 1 string (NEON)](https://lemire.me/blog/2023/05/15/computing-the-utf-8-size-of-a-latin-1-string-quickly-arm-neon-edition/)
* [Computing the UTF-8 size of a Latin 1 string (AVX)](https://lemire.me/blog/2023/02/16/computing-the-utf-8-size-of-a-latin-1-string-quickly-avx-edition/)
* [Counting the number of matching chars in two ASCII strings](https://lemire.me/blog/2021/05/21/counting-the-number-of-matching-characters-in-two-ascii-strings)

# Group inclusion
* [String belongs to a small set](https://lemire.me/blog/2022/12/30/quickly-checking-that-a-string-belongs-to-a-small-set)
* [Absence of a string (AVX)](https://lemire.me/blog/2022/12/15/checking-for-the-absence-of-a-string-naive-avx-512-edition)

# Validate
* [Unicode validation](https://lemire.me/blog/2020/10/20/ridiculously-fast-unicode-utf-8-validation/)
