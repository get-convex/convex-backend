//! Encoding Rust types into configuration file formats
use std::marker::PhantomData;

use prost_reflect::{
    text_format::FormatOptions,
    DynamicMessage,
    MessageDescriptor,
};

use super::decoding::ProstMessage;

/// Encodes a Prost message to textproto format. This is the inverse of
/// [`super::decoding::TextProtoDecoder`].
#[derive(Clone)]
pub struct TextProtoEncoder<T: ProstMessage> {
    descriptor: MessageDescriptor,
    _type: PhantomData<T>,
}

impl<T: ProstMessage> TextProtoEncoder<T> {
    pub const fn new(descriptor: MessageDescriptor) -> Self {
        Self {
            descriptor,
            _type: PhantomData,
        }
    }

    pub fn encode(&self, value: &T) -> anyhow::Result<Vec<u8>> {
        let mut message = DynamicMessage::new(self.descriptor.clone());
        message.transcode_from(value)?;
        let text = message.to_text_format_with_options(&FormatOptions::new().pretty(true));
        Ok(text.into_bytes())
    }
}
