use serde::{de, Deserialize, Deserializer};
use crate::primitives::{Bytes, U8};

impl<'de> Deserialize<'de> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>,
    {
        let s = extract_hex_string(deserializer)?;

        let vec: Vec<u8> = hex::decode(&s)
            .map_err(|_| de::Error::custom(format!("Invalid hex string: {}", s)))?;

        Ok(Bytes(vec))
    }
}

impl<'de> Deserialize<'de> for U8 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>,
    {
        let s = extract_hex_string(deserializer)?;
    
        let byte = u8::from_str_radix(&s, 16)
            .map_err(|_| de::Error::custom(format!("Invalid hex string: {}", s)))?;

        Ok(Self(byte))
    }
}

fn extract_hex_string<'de, D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if let Some(stripped) = s.strip_prefix("0x") {
        Ok(match stripped.len() % 2 { 
            0 => stripped.to_owned(),
            _ => "0".to_owned() + &stripped,
        })
    } else {
        Err(de::Error::custom(format!("Hex string must start with 0x: {}", s)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deserialize_bytes() {
        let s = r#""0x1234""#;
        let bytes: Bytes = serde_json::from_str(s).unwrap();
        assert_eq!(bytes.0, vec![0x12, 0x34]);

        let s = r#""0x21234""#;
        let bytes: Bytes = serde_json::from_str(s).unwrap();
        assert_eq!(bytes.0, vec![0x02, 0x12, 0x34]);

        let s = r#""0x0""#;
        let bytes: Bytes = serde_json::from_str(s).unwrap();
        assert_eq!(bytes.0, vec![0x00]);
    }

    #[test]
    fn deserialize_u8() {
        let s = r#""0x11""#;
        let byte: U8 = serde_json::from_str(s).unwrap();
        assert_eq!(byte.0, 0x11);

        let s = r#""0x1""#;
        let byte: U8 = serde_json::from_str(s).unwrap();
        assert_eq!(byte.0, 0x01);

        let s = r#""0x0""#;
        let byte: U8 = serde_json::from_str(s).unwrap();
        assert_eq!(byte.0, 0x00);
    }
}

