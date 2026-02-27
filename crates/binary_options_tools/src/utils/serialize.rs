pub mod bool2int {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let num = if *value { 1 } else { 0 };
        serializer.serialize_u8(num)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        let num = u8::deserialize(deserializer)?;
        Ok(num != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::bool2int;
    use serde::{Deserialize, Serialize};
    use serde_json;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        #[serde(with = "bool2int")]
        flag: bool,
    }

    #[test]
    fn test_bool2int_serialize() {
        let s1 = TestStruct { flag: true };
        let j1 = serde_json::to_string(&s1).unwrap();
        assert_eq!(j1, "{\"flag\":1}");

        let s2 = TestStruct { flag: false };
        let j2 = serde_json::to_string(&s2).unwrap();
        assert_eq!(j2, "{\"flag\":0}");
    }

    #[test]
    fn test_bool2int_deserialize() {
        let j1 = "{\"flag\":1}";
        let s1: TestStruct = serde_json::from_str(j1).unwrap();
        assert!(s1.flag);

        let j2 = "{\"flag\":0}";
        let s2: TestStruct = serde_json::from_str(j2).unwrap();
        assert!(!s2.flag);

        let j3 = "{\"flag\":2}";
        let s3: TestStruct = serde_json::from_str(j3).unwrap();
        assert!(s3.flag);
    }
}
