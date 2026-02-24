use common::{
    document::ResolvedDocument,
    knobs::MYSQL_DOCUMENT_ENCODING,
};
use value::TabletId;

/// v0 encoding: internal JSON
mod v0 {
    use common::document::ResolvedDocument;
    use value::TabletId;
    pub(crate) fn decode(
        bytes: &[u8],
        table: TabletId,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        if bytes == b"null" {
            return Ok(None);
        }
        let value = value::json_deserialize_bytes(bytes)?;
        Ok(Some(ResolvedDocument::from_database(table, value)?))
    }
    pub(crate) fn encode(maybe_doc: Option<&ResolvedDocument>) -> anyhow::Result<Vec<u8>> {
        // TODO: we should not bother encoding None documents
        let json_str = match maybe_doc {
            Some(document) => document.value().json_serialize()?,
            None => serde_json::Value::Null.to_string(),
        };
        Ok(json_str.into_bytes())
    }
}

/// v1 encoding: ConvexValue sort key -> block LZ4 with custom dictionary ->
/// prepend version tag & size
mod v1 {
    use bytes::Buf;
    use common::document::ResolvedDocument;
    use value::{
        sorting::write_sort_key,
        walk::ConvexValueType,
        ConvexValue,
        Size,
        TabletId,
    };
    pub(crate) const VERSION_TAG: u8 = 1;

    // Tiny custom dictionary for LZ4 compression. This is required for decoding
    // - don't change this! Every document has a _creationTime and _id field,
    // and we q throw in some JSON at the front because it doesn't cost anything
    // and some system documents have internal JSON embedded in them
    const DICT: &[u8] = b"{\"$integer\":\"AAAAAAAAAAA=\"},\x00_id\x00\x10\x15_creationTime\x00\x0d";

    pub(crate) fn decode(
        mut bytes: &[u8],
        table: TabletId,
    ) -> anyhow::Result<Option<ResolvedDocument>> {
        anyhow::ensure!(
            bytes.try_get_u8()? == VERSION_TAG,
            "not a v1 encoded document"
        );
        let uncompressed_size = bytes.try_get_u32()?;
        let decompressed =
            lz4_flex::block::decompress_with_dict(bytes, uncompressed_size.try_into()?, DICT)?;
        let mut reader = &decompressed[..];
        let value = ConvexValue::read_sort_key(&mut reader)?;
        anyhow::ensure!(reader.is_empty(), "trailing bytes");
        Ok(Some(ResolvedDocument::from_database(table, value)?))
    }
    pub(crate) fn encode(maybe_doc: Option<&ResolvedDocument>) -> anyhow::Result<Vec<u8>> {
        // TODO: we should not bother encoding None documents
        let value = match maybe_doc {
            Some(document) => document.value(),
            None => return Ok(vec![]),
        };
        let mut sort_key = Vec::with_capacity(value.size());
        let Ok(()) = write_sort_key::<_, true>(
            ConvexValueType::<&ConvexValue>::Object(value),
            &mut sort_key,
        );
        let mut compressed =
            vec![0u8; 5 + lz4_flex::block::get_maximum_output_size(sort_key.len())];
        compressed[0] = VERSION_TAG;
        compressed[1..5].copy_from_slice(&u32::try_from(sort_key.len())?.to_be_bytes());
        let compressed_len =
            lz4_flex::block::compress_into_with_dict(&sort_key, &mut compressed[5..], DICT)?;
        compressed.truncate(5 + compressed_len);
        Ok(compressed)
    }

    #[cfg(test)]
    mod tests {
        use cmd_util::env::env_config;
        use proptest::prelude::*;
        use value::assert_val;

        #[test]
        fn test_frozen_document() {
            let doc = ResolvedDocument::from_database(
                TabletId::MIN,
                assert_val!({
                    "_id" => "2pj577m1qs11bje4wwb6abkt9wa3m6g",
                    "_creationTime" => 1770416476.678,
                    "null" => null,
                    "int64" => 0x1234,
                    "float_normal" => std::f64::consts::E,
                    "float_nan" => f64::NAN,
                    "bool" => true,
                    "string" => "a".repeat(1000),
                    "array" => [1,2,3,4, {"foo"=>"bar"}],
                }),
            )
            .unwrap();
            let frozen_encoding = b"\x01\0\0\x04\x88\x0c\x10\0\x81\xc1\xdaa\x9aW+dZ-\0\xf0A2pj577m1qs11bje4wwb6abkt9wa3m6g\0array\0\x12\t\x01\t\x02\t\x03\t\x04\x15foo\0\x10bar\0\0\0bool\0\x0ffloat_nan\0\r\xff\xf8\0\0\x02\0\x03\x13\0\xff\x19ormal\0\r\xc0\x05\xbf\n\x8b\x14Wiint64\0\n\x124null\0\x03string\0\x10aa\x02\0\xff\xff\xff\xd2`aaaa\0\0";
            assert_eq!(
                decode(frozen_encoding, doc.id().tablet_id).unwrap(),
                Some(doc)
            );
        }

        use super::*;
        proptest! {
            #![proptest_config(
                ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
            )]

            #[test]
            fn test_roundtrip(doc in any::<ResolvedDocument>()) {
                let table_id = doc.id().tablet_id;
                assert_eq!(decode(&encode(Some(&doc)).unwrap(), table_id).unwrap(), Some(doc));
            }
        }
    }
}

pub(crate) fn encode(maybe_doc: Option<&ResolvedDocument>) -> anyhow::Result<Vec<u8>> {
    match *MYSQL_DOCUMENT_ENCODING {
        0 => v0::encode(maybe_doc),
        1 => v1::encode(maybe_doc),
        x => anyhow::bail!("Unknown encoding version {x}"),
    }
}
pub(crate) fn decode(bytes: &[u8], table: TabletId) -> anyhow::Result<Option<ResolvedDocument>> {
    match bytes.first() {
        None => Ok(None),
        Some(&b'{' | &b'n') => v0::decode(bytes, table),
        Some(&v1::VERSION_TAG) => v1::decode(bytes, table),
        Some(x) => anyhow::bail!("Unknown encoding version {x}"),
    }
}
