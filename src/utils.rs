use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;

pub fn timestamp() -> i64 {
    Utc::now().timestamp_nanos()
}

pub fn timestamp_from_str(datetime: &str) -> Result<i64, String> {
    DateTime::parse_from_rfc3339(datetime)
        .map(|time| time.timestamp_millis())
        .map_err(|error| format!("could not parse datetime string {}: {:?}", datetime, error))
}

pub fn deserialize_from_bytes<T: DeserializeOwned>(bytes: Vec<u8>) -> Result<Option<T>, String> {
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("could not deserialize from string: {:?}", error))
}

#[cfg(test)]
mod tests {
    use super::{timestamp, timestamp_from_str};

    #[test]
    fn test_timestamp() {
        println!("timestamp: {}", timestamp());
    }

    #[test]
    fn test_timestamp_from_str() {
        let time_str = "2022-03-24T02:46:46.000Z";
        let time = timestamp_from_str(time_str).unwrap();
        assert_eq!(1648090006000, time);
    }
}
