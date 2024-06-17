//! We encode DocumentIds in two steps. First, we encode them into binary:
//! ```text
//! document_id = [ VInt(table_number) ] [ internal ID ] [ footer ]
//! ```
//! We use VInt encoding for the table number, which uses between one and five
//! bytes. Then, we write the 16 bytes of the internal ID as is. Finally, the
//! footer is a checksum of the ID so far XOR'd with the version number.
//! ```text
//! footer = fletcher16( [ VInt(table_number) ] [ internal ID ] ) ^ version
//! ```
use std::{
    cmp,
    str::FromStr,
};

use thiserror::Error;

pub use crate::document_id::DeveloperDocumentId;
use crate::{
    base32::{
        self,
        clamp_to_alphabet,
        InvalidBase32Error,
    },
    table_name::TableNumber,
    InternalId,
    ResolvedDocumentId,
    TabletIdAndTableNumber,
};

// The table number is encoded in one to five bytes with VInt encoding.
const MIN_TABLE_NUMBER_LEN: usize = 1;
const MAX_TABLE_NUMBER_LEN: usize = 5;

// The internal ID is always 16 bytes.
const INTERNAL_ID_LEN: usize = 16;

// The footer is always two bytes and includes a Fletcher16 checksum of the rest
// of the ID XOR'd with the version number.
const FOOTER_LEN: usize = 2;
const VERSION: u16 = 0;

const MIN_BINARY_LEN: usize = MIN_TABLE_NUMBER_LEN + INTERNAL_ID_LEN + FOOTER_LEN;
const MIN_BASE32_LEN: usize = base32::encoded_len(MIN_BINARY_LEN);

const MAX_BINARY_LEN: usize = MAX_TABLE_NUMBER_LEN + INTERNAL_ID_LEN + FOOTER_LEN;
const MAX_BASE32_LEN: usize = base32::encoded_len(MAX_BINARY_LEN);

#[derive(Debug, Error)]
pub enum IdDecodeError {
    #[error("Unable to decode ID: ID wasn't valid base32")]
    InvalidBase32(#[from] InvalidBase32Error),
    #[error("Unable to decode ID: Invalid ID length {0}")]
    InvalidLength(usize),
    #[error("Unable to decode ID: Invalid table number")]
    InvalidTableNumber(#[from] VintDecodeError),
    #[error("Unable to decode ID: Invalid table number")]
    ZeroTableNumber,
    #[error("Unable to decode ID: Invalid ID version {0} (expected {1})")]
    InvalidIdVersion(u16, u16),
}

impl DeveloperDocumentId {
    pub fn encoded_len(&self) -> usize {
        let byte_length = vint_len((*self.table()).into()) + 16 + 2;
        base32::encoded_len(byte_length)
    }

    pub fn encode(&self) -> String {
        let mut buf = [0; MAX_BINARY_LEN];

        let mut pos = 0;

        pos += vint_encode((*self.table()).into(), &mut buf[pos..]);

        buf[pos..(pos + 16)].copy_from_slice(&self.internal_id());
        pos += 16;

        let footer = fletcher16(&buf[..pos]) ^ VERSION;
        buf[pos..(pos + 2)].copy_from_slice(&footer.to_le_bytes());
        pos += 2;

        base32::encode(&buf[..pos])
    }

    /// Is the given string an ID that's not in its canonical encoding?
    pub fn is_noncanonical_id(s: &str) -> bool {
        let Ok(id) = Self::decode(s) else {
            return false;
        };
        s != id.encode()
    }

    pub fn decode(s: &str) -> Result<Self, IdDecodeError> {
        // NB: We want error paths to be as quick as possible, even if `s` is very long.
        // So, be sure to do the length check before decoding the base32.
        if s.len() < MIN_BASE32_LEN || MAX_BASE32_LEN < s.len() {
            return Err(IdDecodeError::InvalidLength(s.len()));
        }

        let buf = base32::decode(s)?;

        let mut pos = 0;

        let (table_number, bytes_read) = vint_decode(&buf[pos..])?;
        pos += bytes_read;
        let Ok(table_number) = TableNumber::try_from(table_number) else {
            return Err(IdDecodeError::ZeroTableNumber);
        };

        let internal_id = buf
            .get(pos..(pos + 16))
            .ok_or(IdDecodeError::InvalidLength(s.len()))?
            .try_into()
            .expect("Slice wasn't length 16?");
        pos += 16;

        let expected_footer = fletcher16(&buf[..pos]) ^ VERSION;

        let footer_bytes = buf
            .get(pos..(pos + 2))
            .ok_or(IdDecodeError::InvalidLength(s.len()))?
            .try_into()
            .expect("Slice wasn't length 2?");
        let footer = u16::from_le_bytes(footer_bytes);
        pos += 2;

        if expected_footer != footer {
            return Err(IdDecodeError::InvalidIdVersion(footer, expected_footer));
        }

        // Sanity check that we used all of our input bytes.
        if pos != buf.len() {
            return Err(IdDecodeError::InvalidLength(s.len()));
        }

        let id = DeveloperDocumentId::new(table_number, internal_id);

        // Check that decoding was one-to-one.
        // TODO: Checking base32 decoding above alone isn't sufficient, see
        // `test_id_decoding_one_to_one` below for a counterexample if we only check
        // that `base32::decode` is one-to-one.
        if id.encode() != s {
            return Err(IdDecodeError::InvalidLength(s.len()));
        }

        Ok(id)
    }

    pub fn to_resolved(
        &self,
        f: impl Fn(TableNumber) -> anyhow::Result<TabletIdAndTableNumber>,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let table_id = f(*self.table())?;
        Ok(ResolvedDocumentId {
            tablet_id: table_id.tablet_id,
            developer_id: *self,
        })
    }

    /// Decode a string to the closest valid ID with the given table number.
    /// i.e. if s = id.encoded(), then decode_lossy(s) = id.
    /// and if s > id.encoded(), then decode_lossy(s) >= id.
    /// and if s < id.encoded(), then decode_lossy(s) <= id.
    fn decode_lossy(s: &str, table_number: TableNumber) -> Self {
        let in_base32_alphabet: String = clamp_to_alphabet(s, MAX_BASE32_LEN);
        let buf = base32::decode(&in_base32_alphabet)
            .expect("all characters should be in the base32 alphabet");

        let encoded_table_number = {
            let mut table_number_buf = [0; MAX_BINARY_LEN];
            let pos = vint_encode(table_number.into(), &mut table_number_buf[..]);
            table_number_buf[..pos].to_vec()
        };
        let Some(internal_id_buf) = buf.strip_prefix(&*encoded_table_number) else {
            // It doesn't start with the table number, so it's either before or after the ID
            // space.
            if buf < encoded_table_number {
                return Self::new(table_number, InternalId::MIN);
            } else {
                return Self::new(table_number, InternalId::MAX);
            }
        };
        // Pad with 0s if the internal ID is too short, and truncate if too long.
        let internal_id = {
            let mut internal_id = [0; 16];
            let truncated = &internal_id_buf[..cmp::min(16, internal_id_buf.len())];
            internal_id[..truncated.len()].copy_from_slice(truncated);
            internal_id.to_vec()
        };
        // Note we can ignore the footer because it's deterministic based on internal
        // id.
        Self::new(
            table_number,
            internal_id.try_into().expect("internal ID is 16 bytes"),
        )
    }

    /// `s` is a string in the ID space with virtual table number.
    /// Map it to a string in the ID space with physical table number,
    /// where ordering of the string relative to the ID space is preserved.
    ///
    /// i.e. if `id_virtual` is any ID in the virtual ID space, and
    /// `id_physical` is the corresponding physical ID, then
    /// s.cmp(id_virtual) ==
    /// map_string_between_table_numbers(s).cmp(id_physical).
    pub fn map_string_between_table_numbers(
        s: &str,
        virtual_table_number_map: VirtualTableNumberMap,
    ) -> String {
        let decoded_lossy = Self::decode_lossy(s, virtual_table_number_map.virtual_table_number);
        let reencoded = decoded_lossy.encode();
        let encoded_dest = Self::new(
            virtual_table_number_map.physical_table_number,
            decoded_lossy.internal_id(),
        )
        .encode();
        match s.cmp(&reencoded) {
            cmp::Ordering::Equal => {
                // If s == reencoded, the decode was lossless.
                encoded_dest
            },
            cmp::Ordering::Less => {
                // If s < reencoded, adjust the string to be barely before encoded_dest.
                // Decrement the last character and append '~' which is after the base32
                // alphabet.
                let all_but_last = &encoded_dest[..encoded_dest.len() - 1];
                // encoded ID is nonempty and characters are in the base32 alphabet.
                let last_decremented =
                    ((encoded_dest.chars().last().expect("encoded ID is nonempty") as u8) - 1)
                        as char;
                format!("{all_but_last}{last_decremented}~")
            },
            cmp::Ordering::Greater => {
                // If s > reencoded, adjust the string to be barely after encoded_dest.
                // Append '+' which is before the base32 alphabet.
                format!("{encoded_dest}+")
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VirtualTableNumberMap {
    pub virtual_table_number: TableNumber,
    pub physical_table_number: TableNumber,
}

impl From<ResolvedDocumentId> for DeveloperDocumentId {
    fn from(document_id: ResolvedDocumentId) -> Self {
        document_id.developer_id
    }
}

impl FromStr for DeveloperDocumentId {
    type Err = IdDecodeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        DeveloperDocumentId::decode(s)
    }
}

// Encode `n` with VInt encoding to `out`, returning the number of bytes
// written.
fn vint_encode(mut n: u32, out: &mut [u8]) -> usize {
    let mut pos = 0;
    loop {
        // If `n` has seven or fewer bits, we're done.
        if n < 0b1000_0000 {
            out[pos] = n as u8;
            pos += 1;
            break;
        }
        // Otherwise, emit the lowest seven bits with the continuation bit set.
        else {
            out[pos] = ((n & 0b0111_1111) | 0b1000_0000) as u8;
            pos += 1;
            n >>= 7;
        }
    }
    pos
}

// Compute the number of encoded bytes for `n` upfront.
fn vint_len(n: u32) -> usize {
    const ONE_BYTE_MAX: u32 = 1 << 7;
    const TWO_BYTE_MAX: u32 = 1 << 14;
    const THREE_BYTE_MAX: u32 = 1 << 21;
    const FOUR_BYTE_MAX: u32 = 1 << 28;

    match n {
        0..ONE_BYTE_MAX => 1,
        ONE_BYTE_MAX..TWO_BYTE_MAX => 2,
        TWO_BYTE_MAX..THREE_BYTE_MAX => 3,
        THREE_BYTE_MAX..FOUR_BYTE_MAX => 4,
        FOUR_BYTE_MAX.. => 5,
    }
}

#[derive(Debug, Error)]
pub enum VintDecodeError {
    #[error("Integer is too large")]
    TooLarge,
    #[error("Input truncated")]
    Truncated,
}

// Decode a single VInt from `buf`, returning the integer and number of bytes
// read.
fn vint_decode(buf: &[u8]) -> Result<(u32, usize), VintDecodeError> {
    let mut pos = 0;
    let mut n = 0;

    for i in 0.. {
        // If we've consumed more than five bytes, we won't fit in a u32.
        if i >= 5 {
            return Err(VintDecodeError::TooLarge);
        }
        let byte = buf
            .get(pos)
            .map(|b| *b as u32)
            .ok_or(VintDecodeError::Truncated)?;
        pos += 1;

        // Fold in the low seven bits, shifted to their final position.
        n |= (byte & 0b0111_1111) << (i * 7);

        // Stop if the continutation bit isn't set.
        if byte < 0b1000_0000 {
            break;
        }
    }
    Ok((n, pos))
}

// Compute the Fletcher-16 checksum with modulus 256 of `buf`.
//
// [1] Appendix I in https://www.ietf.org/rfc/rfc1145.txt
// [2] https://en.wikipedia.org/wiki/Fletcher%27s_checksum#Fletcher-16
fn fletcher16(buf: &[u8]) -> u16 {
    let mut c0 = 0u8;
    let mut c1 = 0u8;
    for byte in buf {
        c0 = c0.wrapping_add(*byte);
        c1 = c1.wrapping_add(c0);
    }
    ((c1 as u16) << 8) | (c0 as u16)
}

#[cfg(test)]
mod tests {
    use std::cmp;

    use proptest::prelude::*;

    use crate::{
        id_v6::{
            vint_decode,
            vint_encode,
            vint_len,
            VirtualTableNumberMap,
            MAX_BASE32_LEN,
        },
        DeveloperDocumentId,
        InternalId,
        TableNumber,
    };

    #[test]
    fn test_document_id_stability() {
        let mut internal_id = [251u8; 16];
        for i in 1..16 {
            internal_id[i] = internal_id[i - 1].wrapping_mul(251);
        }
        let document_id =
            DeveloperDocumentId::new(1017.try_into().unwrap(), InternalId::from(internal_id));
        assert_eq!(
            document_id.encode(),
            "z43zp6c3e75gkmz1kfwj6mbbx5sw281h".to_string()
        );
    }

    #[test]
    fn test_invalid_table_code() {
        // This string happens to look like an ID with a one byte table code, but the
        // table code ends up taking two bytes, which then causes parsing to
        // fail downstream. This is a regression test where we used to panic in
        // this condition.
        let _ = DeveloperDocumentId::decode("sssswsgggggggggsgcsssfafffsffks");
    }

    #[test]
    fn test_decode_lossy() {
        let real_id = "kg27rxfv99gzp01wmph0gvt92d6hnvy6";
        let decoded = DeveloperDocumentId::decode(real_id).unwrap();
        assert_eq!(decoded.encode(), real_id);
        let decoded_lossy = DeveloperDocumentId::decode_lossy(real_id, *decoded.table());
        assert_eq!(decoded_lossy, decoded);

        // Dropping the last character just affects the footer, so doesn't change the
        // decode_lossy result.
        let decoded_lossy =
            DeveloperDocumentId::decode_lossy(&real_id[..real_id.len() - 1], *decoded.table());
        assert_eq!(decoded_lossy, decoded);

        // Dropping several characters affects the internal id but not the table number.
        let decoded_lossy = DeveloperDocumentId::decode_lossy(&real_id[..10], *decoded.table());
        assert_eq!(decoded_lossy.table(), decoded.table());
        assert!(decoded_lossy < decoded);
        assert!(decoded_lossy.internal_id() > InternalId::MIN);

        // Dropping most characters makes it out of the ID range.
        let decoded_lossy = DeveloperDocumentId::decode_lossy("k", *decoded.table());
        assert_eq!(
            decoded_lossy,
            DeveloperDocumentId::new(*decoded.table(), InternalId::MIN)
        );

        // Increasing the first character makes it out of the ID range in the
        // other direction.
        let decoded_lossy = DeveloperDocumentId::decode_lossy("z", *decoded.table());
        assert_eq!(
            decoded_lossy,
            DeveloperDocumentId::new(*decoded.table(), InternalId::MAX)
        );
    }

    fn test_decode_lossy_ordering(s: &str, id: DeveloperDocumentId) {
        let encoded = id.encode();
        let decoded_lossy = DeveloperDocumentId::decode_lossy(s, *id.table());
        match s.cmp(&encoded) {
            cmp::Ordering::Less => {
                assert!(decoded_lossy <= id);
            },
            cmp::Ordering::Equal => {
                assert_eq!(decoded_lossy, id);
            },
            cmp::Ordering::Greater => {
                assert!(decoded_lossy >= id);
            },
        }
    }

    fn test_map_between_table_numbers(
        s: &str,
        src_id: DeveloperDocumentId,
        dest_table_number: TableNumber,
    ) {
        let dest_id = DeveloperDocumentId::new(dest_table_number, src_id.internal_id());
        let mapped = DeveloperDocumentId::map_string_between_table_numbers(
            s,
            VirtualTableNumberMap {
                virtual_table_number: *src_id.table(),
                physical_table_number: dest_table_number,
            },
        );
        assert_eq!(s.cmp(&src_id.encode()), mapped.cmp(&dest_id.encode()));
    }

    #[test]
    fn test_decode_lossy_trophies() {
        // First character is > base32 alphabet, second character is < base32 alphabet.
        // Regression test for the string getting clamped to "z0000", when it should be
        // clamped to "zzzzz".
        test_decode_lossy_ordering(
            "豈 ",
            "z2bbqng100000000000000000000000004ggy".parse().unwrap(),
        );
        test_decode_lossy_ordering(
            "~",
            "zzzzz40c00000000000000000000000006db2".parse().unwrap(),
        );
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_vint_encode(n in any::<u32>()) {
            let mut buf = [0; 6];
            let written = vint_encode(n, &mut buf);
            assert_eq!(written, vint_len(n));

            let (parsed, read) = vint_decode(&buf).unwrap();
            assert_eq!(read, written);
            assert_eq!(parsed, n);
        }

        #[test]
        fn test_vint_decode(buf in any::<Vec<u8>>()) {
            // Check that decoding never panics.
            let _ = vint_decode(&buf);
        }

        #[test]
        fn proptest_document_idv6(id in any::<DeveloperDocumentId>()) {
            assert_eq!(DeveloperDocumentId::decode(&id.encode()).unwrap(), id);
        }

        #[test]
        fn proptest_encoded_len(id in any::<DeveloperDocumentId>()) {
            assert_eq!(id.encode().len(), id.encoded_len());
        }

        #[test]
        fn proptest_decode_invalid_string(s in any::<String>()) {
            // Check that we don't panic on any input string.
            let _ = DeveloperDocumentId::decode(&s);
        }

        #[test]
        fn proptest_decode_invalid_bytes(bytes in prop::collection::vec(any::<u8>(), 19..=23)) {
            // Generate bytestrings that pass the first few checks in decode to get more code
            // coverage for later panics.
            let _ = DeveloperDocumentId::decode(&crate::base32::encode(&bytes));
        }

        #[test]
        fn proptest_decode_lossy_lossless(id in any::<DeveloperDocumentId>()) {
            test_decode_lossy_ordering(id.encode().as_str(), id);
        }

        #[test]
        fn proptest_decode_lossy_any_str(s in any::<String>(), id in any::<DeveloperDocumentId>()) {
            test_decode_lossy_ordering(&s, id);
        }

        #[test]
        fn proptest_decode_lossy_truncated(
            s in any::<String>(),
            len in 0usize..=MAX_BASE32_LEN,
            id in any::<DeveloperDocumentId>(),
        ) {
            let truncated = s.chars().take(len).collect::<String>();
            test_decode_lossy_ordering(&truncated, id);
        }

        #[test]
        fn proptest_decode_lossy_alphanumeric(s in "[0-9a-z]*", id in any::<DeveloperDocumentId>()) {
            test_decode_lossy_ordering(&s, id);
        }

        #[test]
        fn proptest_map_between_table_numbers_lossless(
            src_id in any::<DeveloperDocumentId>(),
            dest_table_number in any::<TableNumber>(),
        ) {
            test_map_between_table_numbers(&src_id.encode(), src_id, dest_table_number);
        }

        #[test]
        fn proptest_map_between_table_numbers(
            s in "[0-9a-z]*",
            src_id in any::<DeveloperDocumentId>(),
            dest_table_number in any::<TableNumber>(),
        ) {
            test_map_between_table_numbers(&s, src_id, dest_table_number);
        }

        #[test]
        fn proptest_id_decoding_one_to_one(
            s in "[0123456789abcdefghjkmnpqrstvwxyz]{31,37}"
        ) {
            if let Ok(id) = DeveloperDocumentId::decode(&s) {
                assert_eq!(id.encode(), s);
            }
        }
    }

    #[test]
    fn test_id_decoding_one_to_one() {
        let s = "mz1xn7tymdnktmmzqy5xxhn7tjs2nkkfmtjjr";
        DeveloperDocumentId::decode(s).unwrap_err();
    }
}
