use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use color_eyre::eyre::Result;
use serde::{de, Deserialize, Deserializer};

/// Some fields are optional floats that may be empty. Give serde a way to deserialize those.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// An Option<f32> if the field is present, or None if it is not.
pub fn deserialize_option_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("") | None => Ok(None),
        Some(s) => s.parse::<f32>().map(Some).map_err(de::Error::custom),
    }
}

/// Some fields are optional floats that may be empty. Give serde a way to deserialize those.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// An Option<f64> if the field is present, or None if it is not.
pub fn deserialize_option_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("") | None => Ok(None),
        Some(s) => s.parse::<f64>().map(Some).map_err(de::Error::custom),
    }
}

/// Some fields are optional integers that may be empty. Give serde a way to deserialize those.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// An Option<u32> if the field is present, or None if it is not.
pub fn deserialize_option_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("") | None => Ok(None),
        Some(s) => s.parse::<u32>().map(Some).map_err(de::Error::custom),
    }
}

/// Some fields are optional strings that may be empty. Give serde a way to deserialize those.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// An Option<String> if the field is present, or None if it is not.
pub fn deserialize_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt.as_deref() {
        Some("") | None => Ok(None),
        Some(s) => Ok(Some(s.to_string())),
    }
}

/// Deserializer for `DateTime<Utc>` from ISO 8601 strings.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// A DateTime<Utc> if the string is parseable, or an error if it is not.
pub fn deserialize_date_time_utc_from_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(DateTime::parse_from_rfc3339(&s)
        .expect("Invalid RFC3339 string")
        .to_utc())
}

/// Deserializer for `DateTime<FixedOffset>` from ISO 8601 strings.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// A DateTime<FixedOffset> if the string is parseable, or an error if it is not.
pub fn deserialize_date_time_fixed_offset_from_str<'de, D>(
    deserializer: D,
) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(DateTime::parse_from_rfc3339(&s).expect("Invalid RFC3339 string"))
}

/// Deserializer for `DateTime<Utc>` from ISO 8601 strings.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// A DateTime<Utc> if the string is parseable, or an error if it is not.
pub fn deserialize_date_time_utc_from_sec<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let ts = String::deserialize(deserializer)?
        .parse::<i64>()
        .map_err(de::Error::custom)?;
    Ok(DateTime::from_timestamp(ts, 0)
        .expect("Invalid timestamp")
        .to_utc())
}

/// Deserializer for `NaiveDate` from ISO 8601 strings.
/// # Arguments
/// * `deserializer` - The serde deserializer.
/// # Return
/// A NaiveDate if the string is parseable, or an error if it is not.
pub fn deserialize_date_from_str<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(de::Error::custom)
}
