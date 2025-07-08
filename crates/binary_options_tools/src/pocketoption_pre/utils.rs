pub mod float_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.timestamp_millis() as f64 / 1000.0;
        serializer.serialize_f64(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = f64::deserialize(deserializer)?.to_string();
        let (secs, milis) = match s.split_once(".") {
            Some((seconds, miliseconds)) => {
                let secs: i64 = seconds
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                let mut pow = 0;
                if miliseconds.len() <= 9 {
                    pow = 9u32.saturating_sub(miliseconds.len() as u32);
                }
                let milis = miliseconds
                    .parse::<u32>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?
                    * 10i32.pow(pow) as u32;
                (secs, milis)
            }
            None => {
                let secs: i64 = s
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;

                (secs, 0)
            }
        };
        DateTime::from_timestamp(secs, milis)
            .ok_or(serde::de::Error::custom("Error parsing ints to time"))
    }
}

