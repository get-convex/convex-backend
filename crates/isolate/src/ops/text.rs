use anyhow::Context;
use common::runtime::Runtime;
use deno_core::{
    JsBuffer,
    ToJsBuffer,
};
use encoding_rs::{
    CoderResult,
    DecoderResult,
    Encoding,
};
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::{
    json,
    Value as JsonValue,
};

use crate::{
    environment::IsolateEnvironment,
    execution_scope::ExecutionScope,
};

impl<'a, 'b: 'a, RT: Runtime, E: IsolateEnvironment<RT>> ExecutionScope<'a, 'b, RT, E> {
    #[convex_macro::v8_op]
    pub fn op_textEncoder_encode(&mut self, text: String) -> anyhow::Result<ToJsBuffer> {
        Ok(text.into_bytes().into())
    }

    #[convex_macro::v8_op]
    pub fn op_atob(&mut self, mut encoded: String) -> anyhow::Result<JsonValue> {
        encoded.retain(|c| !c.is_ascii_whitespace());
        let bytes = match base64::decode(encoded) {
            Ok(bytes) => bytes,
            Err(err) => return Ok(json!({ "error": err.to_string() })),
        };

        let decoded: String = bytes
            .into_iter()
            .map(|c| std::char::from_u32(c as u32).expect("all u8s are valid characters"))
            .collect();
        Ok(json!({ "decoded": decoded }))
    }

    #[convex_macro::v8_op]
    pub fn op_btoa(&mut self, text: String) -> anyhow::Result<JsonValue> {
        let mut bytes = vec![];
        for char in text.chars() {
            if char as usize > u8::MAX as usize {
                return Ok(
                    json!({ "error": "The string to be encoded contains characters outside of the Latin1 range." }),
                );
            }
            bytes.push(char as u8);
        }
        let encoded = base64::encode(&bytes);
        Ok(json!({ "encoded": encoded }))
    }

    #[convex_macro::v8_op]
    pub fn op_textEncoder_encodeInto(
        &mut self,
        input: String,
        space: f64,
    ) -> anyhow::Result<TextEncodeIntoRetval> {
        let dest_size = space as usize;

        let mut utf16_code_points_read: usize = 0;
        let mut bytes_written: usize = 0;
        for c in input.chars() {
            if bytes_written + c.len_utf8() > dest_size {
                break;
            }
            utf16_code_points_read += c.len_utf16();
            bytes_written += c.len_utf8();
        }
        let bytes = input[0..bytes_written].to_string().into_bytes();
        Ok(TextEncodeIntoRetval {
            bytes: bytes.into(),
            read: utf16_code_points_read,
            written: bytes_written,
        })
    }

    #[convex_macro::v8_op]
    pub fn op_textEncoder_decode(
        &mut self,
        TextDecodeArgs {
            bytes,
            encoding,
            fatal,
            ignoreBOM,
        }: TextDecodeArgs,
    ) -> anyhow::Result<JsonValue> {
        let Some(encoding) = Encoding::for_label(encoding.as_bytes()) else {
            return Ok(
                json!({ "errorRangeError": format!("The encoding label provided ('{}') is invalid.", encoding) }),
            );
        };

        let data = bytes.to_vec();

        let mut decoder = if ignoreBOM {
            encoding.new_decoder_without_bom_handling()
        } else {
            encoding.new_decoder_with_bom_removal()
        };

        let Some(max_buffer_length) = decoder.max_utf8_buffer_length(data.len()) else {
            return Ok(json!({ "error": "Value too large to decode" }));
        };
        let mut output = vec![0; max_buffer_length];

        if fatal {
            let (result, _, written) =
                decoder.decode_to_utf8_without_replacement(data.as_ref(), &mut output, true);
            match result {
                DecoderResult::InputEmpty => {
                    output.truncate(written);
                    let text = std::str::from_utf8(&output).expect("decoded utf8 not valid");
                    Ok(json!({ "text": text }))
                },
                DecoderResult::OutputFull => Ok(json!({ "error": "Provided buffer too small" })),
                DecoderResult::Malformed(..) => {
                    Ok(json!({ "error": "The encoded data is not valid" }))
                },
            }
        } else {
            let (result, _, written, _) = decoder.decode_to_utf8(data.as_ref(), &mut output, true);
            match result {
                CoderResult::InputEmpty => {
                    output.truncate(written);
                    let text = std::str::from_utf8(&output).expect("decoded utf8 not valid");
                    Ok(json!({ "text": text }))
                },
                CoderResult::OutputFull => Ok(json!({ "error": "Provided buffer too small" })),
            }
        }
    }

    #[convex_macro::v8_op]
    pub fn op_textEncoder_normalizeLabel(&mut self, label: String) -> anyhow::Result<JsonValue> {
        let Some(encoding) = Encoding::for_label_no_replacement(label.as_bytes()) else {
            return Ok(
                json!({ "error": format!("The encoding label provided ('{}') is invalid.", label) }),
            );
        };
        Ok(json!({"label": encoding.name().to_lowercase()}))
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TextDecodeArgs {
    bytes: JsBuffer,
    encoding: String,
    fatal: bool,
    ignoreBOM: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TextEncodeIntoRetval {
    bytes: ToJsBuffer,
    read: usize,
    written: usize,
}
