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

pub fn encode(data: &[u8]) -> String {
    let mut out = Vec::with_capacity((data.len() + 4) / 5 * 5);

    // Process the input in chunks of length 5 (i.e 40 bits), potentially padding
    // the last chunk with zeros for now.
    for chunk in data.chunks(5) {
        let mut block = [0u8; 5];
        block[..chunk.len()].copy_from_slice(chunk);

        let mut indexes = [0u8; 8];

        // Turn our block of 5 groups of 8 bits into 8 groups of 5 bits.
        indexes[0] = (block[0] & 0b1111_1000) >> 3;
        indexes[1] = ((block[0] & 0b0000_0111) << 2) | ((block[1] & 0b1100_0000) >> 6);
        indexes[2] = (block[1] & 0b0011_1110) >> 1;
        indexes[3] = ((block[1] & 0b0000_0001) << 4) | ((block[2] & 0b1111_0000) >> 4);
        indexes[4] = ((block[2] & 0b0000_1111) << 1) | (block[3] >> 7);
        indexes[5] = (block[3] & 0b0111_1100) >> 2;
        indexes[6] = (block[3] & 0b0000_0011) << 3 | ((block[4] & 0b1110_0000) >> 5);
        indexes[7] = block[4] & 0b0001_1111;

        // Look up each 5 bit index in our alphabet.
        for index in indexes {
            out.push(ALPHABET[index as usize]);
        }
    }
    // Truncate any extra zeros we added on the last block.
    if data.len() % 5 != 0 {
        let num_extra = 8 - (data.len() % 5 * 8 + 4) / 5;
        out.truncate(out.len() - num_extra);
    }
    String::from_utf8(out).unwrap()
}

#[derive(Debug, Error)]
#[error("Invalid character {character} at position {position} in {string}")]
pub struct InvalidBase32Error {
    pub character: char,
    pub position: usize,
    pub string: String,
}

pub fn decode(data: &str) -> Result<Vec<u8>, InvalidBase32Error> {
    let data_bytes = data.as_bytes();
    let out_length = data_bytes.len() * 5 / 8;
    let mut out = Vec::with_capacity((out_length + 4) / 5 * 5);

    // Process the data in 8 byte chunks, reversing the encoding process.
    for chunk in data_bytes.chunks(8) {
        let mut indexes = [0u8; 8];
        for (i, byte) in chunk.iter().enumerate() {
            // Invert the alphabet mapping to recover `indexes`.
            let offset = match *byte {
                b'0'..=b'9' => b'0',
                b'a'..=b'h' => b'a' - 10,
                b'j'..=b'k' => b'a' - 10 + 1,
                b'm'..=b'n' => b'a' - 10 + 2,
                b'p'..=b't' => b'a' - 10 + 3,
                b'v'..=b'z' => b'a' - 10 + 4,
                _ => {
                    return Err(InvalidBase32Error {
                        character: data.chars().nth(i).unwrap_or_else(|| {
                            panic!("Checked characters 0..{i} in {data} were one-byte")
                        }),
                        position: i,
                        string: data.to_string(),
                    })
                },
            };
            indexes[i] = byte - offset;
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
    use proptest::prelude::*;

    use super::*;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
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
}
