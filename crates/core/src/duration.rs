//! Human-readable durations (`"30s"`, `"10m"`, `"1h 30m"`) for config and flags.
//!
//! Use `#[serde(with = "magpie_core::duration")]` on `Duration` fields, and the
//! `*_option` functions with `serialize_with`/`deserialize_with` on `Option<Duration>`.

use std::time::Duration;

use serde::{Deserialize, Deserializer, Serializer};

/// Parses a duration. Works as a clap `value_parser`.
///
/// # Errors
///
/// Returns [`humantime::DurationError`] for invalid text.
pub fn parse(text: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(text)
}

/// Serializes a duration as text.
///
/// # Errors
///
/// Returns the serializer's error.
pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&humantime::format_duration(*value))
}

/// Deserializes a duration from text.
///
/// # Errors
///
/// Returns an error for a non-string or invalid duration.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    parse(&text).map_err(serde::de::Error::custom)
}

/// Serializes `Some` as text, `None` as none.
///
/// # Errors
///
/// Returns the serializer's error.
pub fn serialize_option<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(duration) => serialize(duration, serializer),
        None => serializer.serialize_none(),
    }
}

/// Deserializes an optional duration.
///
/// # Errors
///
/// Returns an error for an invalid duration.
pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)?
        .map(|text| parse(&text).map_err(serde::de::Error::custom))
        .transpose()
}
