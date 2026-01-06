use serde::{
    Deserialize,
    Serialize,
};

#[derive(Serialize, Deserialize, Copy, Clone, Debug)]
pub enum Region {
    #[serde(rename = "aws-us-east-1")]
    AwsUsEast1,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_serialization() {
        let region = Region::AwsUsEast1;
        let serialized = serde_json::to_string(&region).unwrap();
        assert_eq!(serialized, "\"aws-us-east-1\"");
    }
}
