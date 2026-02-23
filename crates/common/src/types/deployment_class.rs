use serde::{
    Deserialize,
    Serialize,
};
use utoipa::ToSchema;

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumString,
    strum::Display,
    strum::VariantArray,
    strum::IntoStaticStr,
    Serialize,
    Deserialize,
    ToSchema,
)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum DeploymentClass {
    S16,
    S256,
    D1024,
}
