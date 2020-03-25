pub const DATE_TIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

pub mod opt_datetime {
    use super::DATE_TIME_FORMAT;
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(date) = date {
            let s = format!("{}", date.format(DATE_TIME_FORMAT));
            serializer.serialize_str(&s)
        } else {
            serializer.serialize_str("")
        }
    }

    pub fn deserialize<'d, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        if s == "" {
            Ok(None)
        } else {
            let date = Utc.datetime_from_str(&s, DATE_TIME_FORMAT).map_err(Error::custom)?;
            Ok(Some(date))
        }
    }
}

pub mod datetime {
    use super::DATE_TIME_FORMAT;
    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(DATE_TIME_FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'d, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'d>,
    {
        let s = String::deserialize(deserializer)?;
        Utc.datetime_from_str(&s, DATE_TIME_FORMAT).map_err(Error::custom)
    }
}

pub mod hashset_list {
    use serde::{self, de::Error, Deserialize, Deserializer, Serializer};
    use std::collections::HashSet;
    use std::fmt::Debug;
    use std::hash::Hash;
    use std::str::FromStr;

    pub fn serialize<T, S>(set: &HashSet<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: ToString,
        S: Serializer,
    {
        let s: Vec<_> = set.iter().map(|e| e.to_string()).collect();
        let s = s.join(",");
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'d, D, T>(deserializer: D) -> Result<HashSet<T>, D::Error>
    where
        D: Deserializer<'d>,
        T: FromStr + Eq + Hash,
        <T as FromStr>::Err: Debug,
    {
        let s = String::deserialize(deserializer)?;
        let mut set = HashSet::new();
        for v in s.split(",").map(|v| v.trim()) {
            let v = v
                .parse::<T>()
                .map_err(|err| Error::custom(format!("parse error: {:?}", err)))?;
            set.insert(v);
        }
        Ok(set)
    }
}
