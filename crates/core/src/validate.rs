//! Shared `garde` rules, so every config reports problems the same way.

use std::time::Duration;

/// Rule: the field may be set only when `other` is set. Pair it with the reverse rule on `other`.
pub fn requires<'a, T, U>(
    other: Option<&'a U>,
    other_name: &'static str,
) -> impl FnOnce(&Option<T>, &()) -> garde::Result + use<'a, T, U> {
    move |value, ()| {
        if value.is_some() && other.is_none() {
            Err(garde::Error::new(format!("requires `{other_name}`")))
        } else {
            Ok(())
        }
    }
}

/// Rule: the duration is not zero.
///
/// # Errors
///
/// Returns an error for a zero duration.
pub fn non_zero(value: &Duration, (): &()) -> garde::Result {
    if value.is_zero() {
        Err(garde::Error::new("must be greater than zero"))
    } else {
        Ok(())
    }
}
