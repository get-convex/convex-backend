use std::collections::BTreeMap;

use convex_fivetran_common::fivetran_sdk::DataType as FivetranDataType;

use crate::api_types::FivetranFieldName;

#[derive(derive_more::From, Clone)]
pub struct FivetranTableSchema(pub BTreeMap<FivetranFieldName, FivetranDataType>);
