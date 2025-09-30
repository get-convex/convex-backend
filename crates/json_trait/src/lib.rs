#![feature(trait_alias)]
use std::fmt::Debug;

use anyhow::Context as _;
use errors::ErrorMetadata;
use serde::{
    de::DeserializeOwned,
    Serialize,
};

pub fn invalid_json() -> ErrorMetadata {
    ErrorMetadata::bad_request("InvalidJson", "Invalid JSON")
}

/// The standard serde idiom is to directly implement `Serialize` and
/// `Deserialize` on types to make them JSON-serializable.
/// However, this has some downsides, e.g. making it hard to let the JSON & Rust
/// structures differ from each other, and it also means that any custom
/// validation logic has to fit into Serde's error model - which collapses our
/// typed `ErrorMetadata` errors into strings.
///
/// Instead we have a pattern of defining our Rust types, and then defining
/// parallel "*Json" types that implement Serialize/Deserialize (usually
/// derived); this does the first layer of serialization; and then we have
/// TryFrom impls in both directions to do any final validation steps.
///
/// This trait makes it possible to name those "*Json" types uniformly.
pub trait JsonForm {
    type Json: Debug;

    fn json_serialize(self) -> anyhow::Result<String>
    where
        Self: JsonSerializable,
    {
        Ok(serde_json::to_string(&<Self::Json>::try_from(self)?)?)
    }

    fn json_deserialize(s: &str) -> anyhow::Result<Self>
    where
        Self: JsonDeserializable,
    {
        Ok(serde_json::from_str::<Self::Json>(s)
            .with_context(invalid_json)?
            .try_into()?)
    }

    /// Deserialize from a `serde_json::Value`.
    ///
    /// This should generally not be preferred because `serde_json::Value` is a
    /// very dynamic and expensive type to construct. Prefer using `Self::Json`
    /// instead.
    fn json_deserialize_value(s: serde_json::Value) -> anyhow::Result<Self>
    where
        Self: JsonDeserializable,
    {
        Ok(serde_json::from_value::<Self::Json>(s)
            .with_context(invalid_json)?
            .try_into()?)
    }
}

pub trait JsonDeserializable = JsonForm
where
    <Self as JsonForm>::Json: DeserializeOwned,
    Self: TryFrom<<Self as JsonForm>::Json>,
    anyhow::Error: From<<Self as TryFrom<<Self as JsonForm>::Json>>::Error>;

pub trait JsonSerializable = JsonForm + Sized
where
    <Self as JsonForm>::Json: Serialize + TryFrom<Self>,
    anyhow::Error: From<<<Self as JsonForm>::Json as TryFrom<Self>>::Error>;
