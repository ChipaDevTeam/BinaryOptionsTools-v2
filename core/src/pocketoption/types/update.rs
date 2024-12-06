use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateStream(Vec<UpdateStreamItem>);



#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateStreamItem {
    active: String,
    #[serde(with = "float_time")]
    time: DateTime<Utc>,
    price: f64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateHistoryNew {
    asset: String,
    period: u32, 
    history: Vec<UpdateCandle>
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateCandle {
    #[serde(with = "float_time")]
    time: DateTime<Utc>,
    price: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBalance {
    is_demo: u32,
    balance: f64
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UpdateAssets(Vec<OptionData>);

#[derive(Debug, Deserialize, Serialize)]
struct OptionData {
    id: i32,
    symbol: String,
    name: String,
    asset_type: AssetType,
    in1: i32,
    in2: i32,
    in3: i32,
    in4: i32,
    in5: i32,
    in6: i32,
    in7: i32,
    in8: i32,
    arr: Vec<String>,
    in9: i64,
    val: bool,
    times: Vec<TimeCandle>,
    in10: i32,
    in11: i32,
    in12: i64
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum AssetType {
    Stock,
    Currency,
    Commodity,
    Cryptocurrency,
    Index
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TimeCandle {
    #[serde(with = "duration")]
    time: Duration
}

pub mod float_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        date: &DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.timestamp_millis() as f64 / 1000.0;
        serializer.serialize_f64(s)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = f64::deserialize(deserializer)?.to_string();
        let (secs, milis) = match s.split_once(".") {
            Some((seconds, miliseconds)) => {
                let secs: i64 = seconds.parse::<i64>().map_err(|e| serde::de::Error::custom(e.to_string()))?;
                let mut pow = 0;
                if miliseconds.len() <= 9 {
                    pow = 9u32.saturating_sub(miliseconds.len() as u32);
                } 
                let milis = miliseconds.parse::<u32>().map_err(|e| serde::de::Error::custom(e.to_string()))? * 10i32.pow(pow) as u32;
                (secs, milis)
            },
            None => {
                let secs: i64 = s.parse::<i64>().map_err(|e| serde::de::Error::custom(e.to_string()))?;

                (secs, 0)
            }
        };
        DateTime::from_timestamp(secs, milis).ok_or(serde::de::Error::custom("Error parsing ints to time"))
    }

}

pub mod duration {
    use chrono::Duration;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(
        duration: &Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(duration.num_seconds())
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = i64::deserialize(deserializer)?;
        Ok(Duration::seconds(s))
    }

}


#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    use std::{error::Error, fs::File, io::BufReader};

    #[test]
    fn test_descerialize_update_stream() -> Result<(), Box<dyn Error>> {
        let tests = [
            r#"[["AUS200_otc",1732830010,6436.06]]"#,
            r#"[["AUS200_otc",1732830108.205,6435.96]]"#,
            r#"[["AEDCNY_otc",1732829668.352,1.89817]]"#,
            r#"[["CADJPY_otc",1732830170.793,109.442]]"#,
        ];
        for item in tests.iter() {
            let val: Value = serde_json::from_str(item)?;
            dbg!(&val);
            let res: UpdateStream = serde_json::from_value(val)?;
            dbg!(res);
            // let descerializer = Deserializer::from_str(item).into_iter::<UpdateStream>();
            // for item in descerializer.into_iter() {

            //     let res = item?;
            //     let time_reparsed = serde_json::to_string(&res)?;
            //     dbg!(time_reparsed);
            //     dbg!(res);
            // }
        }        
        Ok(())
    }

    #[test]
    fn test_descerialize_update_history() -> Result<(), Box<dyn Error>> {
        let history_raw = File::open("tests/update_history_new.txt")?;
        let bufreader = BufReader::new(history_raw);
        let history_new: UpdateHistoryNew = serde_json::from_reader(bufreader)?;
        dbg!(history_new);

        Ok(())
    }
}