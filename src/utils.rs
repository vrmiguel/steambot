use serde::{Deserialize, Deserializer};

pub fn deserialize_string_to_usize<'de, D>(deserializer: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.parse::<usize>().map_err(serde::de::Error::custom)
}
