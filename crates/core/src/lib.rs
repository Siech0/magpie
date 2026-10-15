//! Format-neutral domain model: repositories, blobs, permissions, and caching.

mod secret;

pub use secret::{Secret, SecretError, SecretRef};
