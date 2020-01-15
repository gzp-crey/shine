use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

pub fn serialize<S: Serializer>(time: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> {
    time.to_rfc3339().serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<NaiveDateTime, D::Error> {
    let time: String = Deserialize::deserialize(deserializer)?;
    Ok(DateTime<Utc>::parse_from_str(&time, "%Y-%m-%d %H:%M:%S").map_err(D::Error::custom)?)
}
