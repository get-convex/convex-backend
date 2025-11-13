// Forked from https://github.com/andreasots/base32 @ 58909ac.
//
// Copyright (c) 2015 The base32 Developers - MIT License
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
use thiserror::Error;

// Crockford's Base32 alphabet (https://www.crockford.com/base32.html) with lowercase
// alphabetical characters. We also don't decode permissively.
const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";

/// Lookup table for decoding base32 characters
/// Maps ASCII byte values to 5-bit indices (0-31)
/// Invalid characters are marked with 0xFF
const DECODE_TABLE: [u8; 256] = {
    let mut table = [0xFFu8; 256];
    let mut i = 0;
    while i < 32 {
        table[ALPHABET[i] as usize] = i as u8;
        i += 1;
    }
    table
};

pub const fn encoded_len(len: usize) -> usize {
    let last_chunk = match len % 5 {
        0 => 0,
        1 => 2,
        2 => 4,
        3 => 5,
        4 => 7,
        _ => unreachable!(),
    };
    (len / 5) * 8 + last_chunk
}

pub const fn encoded_buffer_len(len: usize) -> usize {
    len.div_ceil(5) * 8
}

/// Writes the base32-encoding of `data` into `out`, which should have length at
/// least `encoded_buffer_len(data.len())`. Only the first
/// `encoded_len(data.len())` bytes of `out` should be used.
#[inline]
pub fn encode_into(out: &mut [u8], data: &[u8]) {
    // Process the input in chunks of length 5 (i.e 40 bits), potentially padding
    // the last chunk with zeros for now.
    for (chunk, out_chunk) in data.chunks(5).zip(out.chunks_mut(8)) {
        let block = chunk.try_into().unwrap_or_else(|_| {
            // Zero-extend the last chunk if necessary
            let mut block = [0u8; 5];
            block[..chunk.len()].copy_from_slice(chunk);
            block
        });

        // Turn our block of 5 groups of 8 bits into 8 groups of 5 bits.
        #[inline]
        fn alphabet(index: u8) -> u8 {
            ALPHABET[index as usize]
        }
        out_chunk[0] = alphabet((block[0] & 0b1111_1000) >> 3);
        out_chunk[1] = alphabet((block[0] & 0b0000_0111) << 2 | ((block[1] & 0b1100_0000) >> 6));
        out_chunk[2] = alphabet((block[1] & 0b0011_1110) >> 1);
        out_chunk[3] = alphabet((block[1] & 0b0000_0001) << 4 | ((block[2] & 0b1111_0000) >> 4));
        out_chunk[4] = alphabet((block[2] & 0b0000_1111) << 1 | (block[3] >> 7));
        out_chunk[5] = alphabet((block[3] & 0b0111_1100) >> 2);
        out_chunk[6] = alphabet((block[3] & 0b0000_0011) << 3 | ((block[4] & 0b1110_0000) >> 5));
        out_chunk[7] = alphabet(block[4] & 0b0001_1111);
    }
}

pub fn encode(data: &[u8]) -> String {
    let mut out = vec![0; encoded_buffer_len(data.len())];
    encode_into(&mut out, data);
    // Truncate any extra zeros we added on the last block.
    out.truncate(encoded_len(data.len()));
    String::from_utf8(out).unwrap()
}

#[derive(Debug, Error, PartialEq)]
#[error("Invalid character {character} at position {position} in {string}")]
pub struct InvalidBase32Error {
    pub character: char,
    pub position: usize,
    pub string: String,
}

pub fn decode(data: &str) -> Result<Vec<u8>, InvalidBase32Error> {
    let data_bytes = data.as_bytes();
    let out_length = data_bytes.len() * 5 / 8;
    let mut out = Vec::with_capacity(out_length.div_ceil(5) * 5);

    // Process the data in 8 byte chunks
    for chunk in data_bytes.chunks(8) {
        let mut indexes = [0u8; 8];
        for (i, byte) in chunk.iter().enumerate() {
            // Safe, bounds-checked, and (after inlining) O(1) lookup
            let index = DECODE_TABLE[*byte as usize];

            if index == 0xFF {
                // Invalid character found
                let position = i + chunk.as_ptr().addr() - data_bytes.as_ptr().addr();
                return Err(InvalidBase32Error {
                    character: data[position..].chars().next().unwrap_or_else(|| {
                        panic!("Checked characters 0..{position} in {data} were one-byte")
                    }),
                    position,
                    string: data.to_string(),
                });
            }
            indexes[i] = index;
        }

        // Regroup our block of 8 5-bit indexes into 5 output bytes.
        out.push((indexes[0] << 3) | (indexes[1] >> 2));
        out.push((indexes[1] << 6) | (indexes[2] << 1) | (indexes[3] >> 4));
        out.push((indexes[3] << 4) | (indexes[4] >> 1));
        out.push((indexes[4] << 7) | (indexes[5] << 2) | (indexes[6] >> 3));
        out.push((indexes[6] << 5) | indexes[7]);
    }

    // Truncate any extra output from our last chunk.
    out.truncate(out_length);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn proptest_base32_encode(bytes in any::<Vec<u8>>()) {
            let encoded = encode(&bytes);
            let expected = base32::encode(base32::Alphabet::Crockford, &bytes);
            assert!(encoded.eq_ignore_ascii_case(&expected));
            assert_eq!(encoded_len(bytes.len()), encoded.len());
        }

        #[test]
        fn proptest_base32_roundtrips(bytes in any::<Vec<u8>>()) {
            assert_eq!(decode(&encode(&bytes)).unwrap(), bytes);
        }

        #[test]
        fn proptest_base32_decode(s in any::<String>()) {
            // Check that decoding never panics on invalid input.
            let _ = decode(&s);
        }

        #[test]
        fn proptest_base32_order_preserving(left in any::<Vec<u8>>(), right in any::<Vec<u8>>()) {
            let left_encoded = encode(&left);
            let right_encoded = encode(&right);
            assert_eq!(left.cmp(&right), left_encoded.cmp(&right_encoded));
        }
    }

    #[test]
    fn test_invalid_base32_error() {
        assert_eq!(
            decode("01234567ë").unwrap_err(),
            InvalidBase32Error {
                character: 'ë',
                position: 8,
                string: "01234567ë".into()
            }
        );
    }
}
