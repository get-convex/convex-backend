use std::collections::BTreeMap;

use common::{
    types::MaybeValue,
    value::{
        ConvexObject,
        FieldName,
    },
};
use serde_json::Value as JsonValue;

/// A object used in patch. Similar to GenericObject but also allows top level
/// undefined fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchValue {
    fields: BTreeMap<FieldName, MaybeValue>,
}

impl PatchValue {
    pub fn apply(self, original: ConvexObject) -> anyhow::Result<ConvexObject> {
        let mut original_fields: BTreeMap<_, _> = original.into();

        for (field, maybe_value) in self.fields {
            match maybe_value.0 {
                Some(value) => {
                    original_fields.insert(field, value);
                },
                None => {
                    original_fields.remove(&field);
                },
            }
        }
        original_fields.try_into()
    }
}

impl From<BTreeMap<FieldName, MaybeValue>> for PatchValue {
    fn from(fields: BTreeMap<FieldName, MaybeValue>) -> Self {
        Self { fields }
    }
}

impl TryFrom<JsonValue> for PatchValue {
    type Error = anyhow::Error;

    fn try_from(json_value: JsonValue) -> anyhow::Result<Self> {
        match json_value {
            JsonValue::Object(map) => {
                let mut fields = BTreeMap::new();
                for (key, value) in map {
                    fields.insert(key.parse()?, MaybeValue::try_from(value)?);
                }
                Ok(fields.into())
            },
            _ => {
                anyhow::bail!("Value must be an Object");
            },
        }
    }
}

impl From<ConvexObject> for PatchValue {
    fn from(obj: ConvexObject) -> Self {
        let mut fields = BTreeMap::new();
        for (field, value) in obj.into_iter() {
            fields.insert(field, MaybeValue::from(value));
        }
        fields.into()
    }
}

#[macro_export]
/// Create an patch object from field/value pairs.
macro_rules! patch_value {
    ($($field:expr => $val:expr),* $(,)?) => {
        {
            use common::types::MaybeValue;
            use $crate::PatchValue;
            use std::collections::BTreeMap;
            #[allow(unused)]
            let mut fields = BTreeMap::new();
            {
                $(
                    fields.insert($field.parse()?, MaybeValue($val));
                )*
            }
            PatchValue::try_from(fields)
        }
    };
}

#[cfg(test)]
mod tests {
    use common::assert_obj;
    use value::{
        assert_val,
        ConvexObject,
    };

    #[test]
    fn test_apply() -> anyhow::Result<()> {
        // Overwrite duplicate fields instead of merging sub-fields.
        let original: ConvexObject = assert_obj!(
            "name" => {
                "first" => "Mr",
                "last" => "Fantastik",
            },
            "job" => "mechanic",
        );
        let patch = patch_value!(
            "name" => Some(assert_val!({
                "first" => "Mr",
                "surname" => "Fantastik",
            })),
            "job" => Some("engineer".try_into()?),
        )?;

        // shallow merge
        let expected = assert_obj!(
            "name" => {
                "first" => "Mr",
                "surname" => "Fantastik",
            },
            "job" => "engineer",
        );
        assert_eq!(patch.apply(original.clone())?, expected);

        // delete a top level field.
        let delete_job = patch_value!("job" => None)?;
        let expected = assert_obj!(
            "name" => {
                "first" => "Mr",
                "last" => "Fantastik",
            },
        );
        assert_eq!(delete_job.apply(original.clone())?, expected);

        // deleting a top level fields that doesn't exist should be a no-op.
        let delete_missing = patch_value!("missing" => None)?;
        let expected = assert_obj!(
            "name" => {
                "first" => "Mr",
                "last" => "Fantastik",
            },
            "job" => "mechanic",
        );
        assert_eq!(delete_missing.apply(original)?, expected);

        Ok(())
    }
}
