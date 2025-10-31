[Daniel Lemire's](https://lemire.me/blog/) and [Wojciech Muła's](http://0x80.pl/) blogs are great resources full of common problems and (very) efficient solutions (most often SIMD-based) encountered in the data processing domain. 
As I was reading the posts, I have observed recurring solutions, potentially you can even call them _patterns_. The goal is to reproduce them on my laptop and gather them in this repo. 

# Conversions

* [ASCII to lower case](https://lemire.me/blog/2024/08/03/converting-ascii-strings-to-lower-case-at-crazy-speeds-with-avx-512)
* [Escaping strings with SIMD](https://lemire.me/blog/2022/09/14/escaping-strings-faster-with-avx-512)
* [Packing a string of digits into an integer](https://lemire.me/blog/2023/07/07/packing-a-string-of-digits-into-an-integer-quickly)
* [Break a long string into lines](https://lemire.me/blog/2024/04/19/how-quickly-can-you-break-a-long-string-into-lines/)
* [Vectorized trimming of line comments](https://lemire.me/blog/2023/04/26/vectorized-trimming-of-line-comments)
* [Serializing IPs](https://lemire.me/blog/2023/02/01/serializing-ips-quickly-in-c)
* [Trimming spaces from strings](https://lemire.me/blog/2023/03/10/trimming-spaces-from-strings-faster-with-sve-on-an-amazon-graviton-3-processor/)
* [Removing chars from strings](https://lemire.me/blog/2022/04/28/removing-characters-from-strings-faster-with-avx-512)
* [Integers to decimal strings](https://lemire.me/blog/2022/03/28/converting-integers-to-decimal-strings-faster-with-avx-512)
* [Binary floating point numbers to integers](https://lemire.me/blog/2021/10/21/converting-binary-floating-point-numbers-to-integers)
* [Integers to fix digits](https://lemire.me/blog/2021/11/18/converting-integers-to-fix-digit-representations-quickly/)
* [Number of digits in integer](https://lemire.me/blog/2021/06/03/computing-the-number-of-digits-of-an-integer-even-faster/)
* [Removing duplicates](https://lemire.me/blog/2017/04/10/removing-duplicates-from-lists-quickly/)

### Decoding / Transcoding / Encoding

* [Decoding Base16 sequences](https://lemire.me/blog/2023/07/27/decoding-base16-sequences-quickly)
* [Latin 1 to UTF-8](https://lemire.me/blog/2023/08/18/transcoding-latin-1-strings-to-utf-8-strings-at-12-gb-s-using-avx-512)
* [UTF-8 to Latin 1](https://lemire.me/blog/2023/08/12/transcoding-utf-8-strings-to-latin-1-strings-at-12-gb-s-using-avx-512)
* [Base16 encoding](https://lemire.me/blog/2022/12/23/fast-base16-encoding)
* [Binary in ASCII](https://lemire.me/blog/2020/05/02/encoding-binary-in-ascii-very-fast)
* [Bitset decoding](https://lemire.me/blog/2022/05/06/fast-bitset-decoding-using-intel-avx-512)

# Searching

* [Control chars with SWAR](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Identifiers with NEON](https://lemire.me/blog/2023/09/04/locating-identifiers-quickly-arm-neon-edition)
* [String prefixes](https://lemire.me/blog/2023/07/14/recognizing-string-prefixes-with-simd-instructions/)
* [JSON escapable chars with SWAR](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Escapable chars with SIMD](https://lemire.me/blog/2025/04/13/detect-control-characters-quotes-and-backslashes-efficiently-using-swar/)
* [Identifying sequence of digits in string](https://lemire.me/blog/2018/09/30/quickly-identifying-a-sequence-of-digits-in-a-string-of-characters)

### HTML Scanning

* [HTML](https://lemire.me/blog/2024/07/05/scan-html-faster-with-simd-instructions-net-c-edition)
* [HTML](https://lemire.me/blog/2024/07/20/scan-html-even-faster-with-simd-instructions-c-and-c)
* [HTML](https://lemire.me/blog/2024/06/08/scan-html-faster-with-simd-instructions-chrome-edition)

# Parsing 

### Numbers
* [Integers](https://lemire.me/blog/2023/09/22/parsing-integers-quickly-with-avx-512)
* [Floating point](https://lemire.me/blog/2021/02/22/parsing-floating-point-numbers-really-fast-in-c)
* [String of digit into integer with SWAR](https://lemire.me/blog/2022/01/21/swar-explained-parsing-eight-digits)

### Timestamps
* [Timestamps](https://lemire.me/blog/2023/07/01/parsing-time-stamps-faster-with-simd-instructions)

### IPs
* [IPs](https://lemire.me/blog/2023/06/08/parsing-ip-addresses-crazily-fast/)

# Filtering
* [Filtering numbers](https://lemire.me/blog/2022/07/14/filtering-numbers-faster-with-sve-on-amazon-graviton-3-processors/)

# Prefix minimum

* [Prefix minimum](https://lemire.me/blog/2023/08/10/coding-of-domain-names-to-wire-format-at-gigabytes-per-second)

# Summarization / Counting
* [Computing the UTF-8 size of a Latin 1 string w/ NEON](https://lemire.me/blog/2023/05/15/computing-the-utf-8-size-of-a-latin-1-string-quickly-arm-neon-edition/)
* [Computing the UTF-8 size of a Latin 1 string w/ AVX](https://lemire.me/blog/2023/02/16/computing-the-utf-8-size-of-a-latin-1-string-quickly-avx-edition/)
* [Counting the number of matching chars in two ASCII strings](https://lemire.me/blog/2021/05/21/counting-the-number-of-matching-characters-in-two-ascii-strings)

# Group inclusion
* [String belongs to a small set](https://lemire.me/blog/2022/12/30/quickly-checking-that-a-string-belongs-to-a-small-set)
* [Absence of a string](https://lemire.me/blog/2022/12/15/checking-for-the-absence-of-a-string-naive-avx-512-edition)

# Validation
* [Unicode validation](https://lemire.me/blog/2020/10/20/ridiculously-fast-unicode-utf-8-validation/)
