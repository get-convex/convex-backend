use convex_fivetran_common::fivetran_sdk::{
    self,
    DataType as FivetranDataType,
};
use convex_fivetran_destination::{
    api_types::FivetranFieldName,
    constants::{
        ID_FIVETRAN_FIELD_NAME,
        SOFT_DELETE_FIVETRAN_FIELD_NAME,
        SYNCED_FIVETRAN_FIELD_NAME,
    },
};
use proptest::{
    arbitrary::any,
    prop_oneof,
    strategy::{
        Just,
        Strategy,
    },
};

pub fn fivetran_field_name_strategy() -> impl Strategy<Value = FivetranFieldName> {
    prop_oneof![
        Just(SYNCED_FIVETRAN_FIELD_NAME.clone()),
        Just(SOFT_DELETE_FIVETRAN_FIELD_NAME.clone()),
        Just(ID_FIVETRAN_FIELD_NAME.clone()),
        proptest::string::string_regex("_?[a-zA-Z][a-zA-Z0-9]*")
            .unwrap()
            .prop_map(|name| name.parse().unwrap()),
    ]
}

fn concrete_fivetran_data_type_strategy() -> impl Strategy<Value = FivetranDataType> {
    prop_oneof![
        // Skipping FivetranDataType::Unspecified
        Just(FivetranDataType::Boolean),
        Just(FivetranDataType::Short),
        Just(FivetranDataType::Int),
        Just(FivetranDataType::Long),
        Just(FivetranDataType::Decimal),
        Just(FivetranDataType::Float),
        Just(FivetranDataType::Double),
        Just(FivetranDataType::NaiveTime),
        Just(FivetranDataType::NaiveDate),
        Just(FivetranDataType::NaiveDatetime),
        Just(FivetranDataType::UtcDatetime),
        Just(FivetranDataType::Binary),
        Just(FivetranDataType::Xml),
        Just(FivetranDataType::String),
        Just(FivetranDataType::Json),
    ]
}

pub fn fivetran_table_strategy() -> impl Strategy<Value = fivetran_sdk::Table> {
    (
        proptest::string::string_regex("[a-zA-Z][a-zA-Z0-9]*").unwrap(),
        proptest::collection::btree_map(
            fivetran_field_name_strategy(),
            (concrete_fivetran_data_type_strategy(), any::<bool>()),
            1..=5,
        )
        .prop_map(|columns| {
            columns
                .into_iter()
                .map(
                    |(field_name, (data_type, in_primary_key))| fivetran_sdk::Column {
                        name: field_name.to_string(),
                        r#type: data_type as i32,
                        primary_key: in_primary_key,
                        decimal: None,
                    },
                )
                .collect()
        }),
    )
        .prop_map(|(name, columns)| fivetran_sdk::Table { name, columns })
}
