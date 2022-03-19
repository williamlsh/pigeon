use chrono::Utc;

pub fn timestamp() -> i64 {
    Utc::now().timestamp_nanos()
}

#[cfg(test)]
mod tests {
    use super::timestamp;

    #[test]
    fn test_timestamp() {
        println!("timestamp: {}", timestamp());
    }
}
