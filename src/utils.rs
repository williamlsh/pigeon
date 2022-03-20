use chrono::Utc;
use serde::de::DeserializeOwned;

pub fn timestamp() -> i64 {
    Utc::now().timestamp_nanos()
}

pub fn deserialize_from_bytes<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<Option<T>, String> {
    match String::from_utf8(bytes) {
        Ok(s) => match serde_json::from_str::<T>(&s) {
            Ok(res) => Ok(Some(res)),
            Err(error) => Err(format!("could not to deserialize from string: {:?}", error)),
        },
        Err(error) => Err(format!("could not to convert bytes to string: {:?}", error)),
    }
}

#[cfg(test)]
mod tests {
    use super::timestamp;

    #[test]
    fn test_timestamp() {
        println!("timestamp: {}", timestamp());
    }
}
