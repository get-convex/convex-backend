//! Decoding configuration files into Rust types
use std::marker::PhantomData;

use prost_reflect::{
    prost::Message,
    DynamicMessage,
    MessageDescriptor,
};

/// Trait representing the decoding of a config file's contents to a generic
/// output type.
pub trait ConfigDecoder: Clone {
    type Output: PartialEq + Clone + Send + Sync + 'static;
    fn decode(&self, contents: Vec<u8>) -> anyhow::Result<Self::Output>;
}

/// Simple decoder for getting the value of the config file as a UTF-8 string.
#[derive(Copy, Clone)]
pub struct TextDecoder;

impl ConfigDecoder for TextDecoder {
    type Output = String;

    fn decode(&self, contents: Vec<u8>) -> anyhow::Result<String> {
        Ok(String::from_utf8(contents)?)
    }
}

/// No-op decoder for getting the raw bytes of the config file.
#[derive(Copy, Clone)]
pub struct BytesDecoder;
impl ConfigDecoder for BytesDecoder {
    type Output = Vec<u8>;

    fn decode(&self, contents: Vec<u8>) -> anyhow::Result<Self::Output> {
        Ok(contents)
    }
}

pub trait ProstMessage = Message + Clone + Default + PartialEq + 'static;

/// Decodes textproto format to a Prost message. You need the
/// [`MessageDescriptor`] for the proto you intend to decode; this can be
/// obtained from the `FILE_DESCRIPTOR_BYTES` constant in every generated
/// protobuf crate.
#[derive(Clone)]
pub struct TextProtoDecoder<T: ProstMessage> {
    descriptor: MessageDescriptor,
    _type: PhantomData<T>,
}

impl<T: ProstMessage> TextProtoDecoder<T> {
    pub const fn new(descriptor: MessageDescriptor) -> Self {
        Self {
            descriptor,
            _type: PhantomData,
        }
    }
}

impl<T: ProstMessage> ConfigDecoder for TextProtoDecoder<T> {
    type Output = T;

    fn decode(&self, contents: Vec<u8>) -> anyhow::Result<T> {
        let contents = std::str::from_utf8(&contents)?;
        let message = DynamicMessage::parse_text_format(self.descriptor.clone(), contents)?;
        Ok(message.transcode_to()?)
    }
}
