use std::collections::BTreeMap;

use crate::{
    api_types::FivetranFieldName,
    fivetran_sdk::DataType as FivetranDataType,
};

#[derive(derive_more::From, Clone)]
pub struct FivetranTableSchema(pub BTreeMap<FivetranFieldName, FivetranDataType>);
