//! Secret support types and functions

use std::{fmt, io, path::PathBuf};

use secrecy::{ExposeSecret as _, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const ENV_PREFIX: &str = "env:";
const FILE_PREFIX: &str = "file:";
const LITERAL_PREFIX: &str = "literal:";

/// Why a secret could not be resolved. Messages name the source, never the value.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SecretError {
    /// The reference string is malformed, e.g. `file:` with no path.
    #[error("invalid secret reference: {reason}")]
    InvalidReference {
        /// What is wrong. Never contains the input, which may be a literal secret.
        reason: &'static str,
    },

    /// The referenced environment variable is not set.
    #[error("environment variable `{name}` is not set")]
    EnvNotSet {
        /// The variable name.
        name: String,
    },

    /// The referenced environment variable is not valid UTF-8.
    #[error("environment variable `{name}` is not valid UTF-8")]
    EnvNotUnicode {
        /// The variable name.
        name: String,
    },

    /// The referenced file does not exist.
    #[error("secret file `{}` does not exist", path.display())]
    FileNotFound {
        /// The file path.
        path: PathBuf,
    },

    /// The process may not read the referenced file.
    #[error("no permission to read secret file `{}`", path.display())]
    FileAccessDenied {
        /// The file path.
        path: PathBuf,
    },

    /// Reading the referenced file failed for another reason.
    #[error("could not read secret file `{}`", path.display())]
    FileRead {
        /// The file path.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// The referenced file is not valid UTF-8.
    #[error("secret file `{}` is not valid UTF-8", path.display())]
    FileNotUtf8 {
        /// The file path.
        path: PathBuf,
        /// The decoding error. It holds only byte offsets, never file content.
        #[source]
        source: std::str::Utf8Error,
    },

    /// The source exists but holds an empty value, which usually means a broken mount or template.
    #[error("secret from `{reference}` is empty")]
    Empty {
        /// The reference, e.g. `env:DB_URL`.
        reference: String,
    },

    /// A secret store (Vault, a cloud manager) failed. The store's own crate defines the source error.
    #[error("secret store failed for `{reference}`")]
    Store {
        /// The reference that failed.
        reference: String,
        /// The store's error.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// A secure string-based secret.
///
/// Memory is zeroed on `Drop` via `Zeroize`. `Debug` is printed as `[REDACTED]` and there is no `Display` implementation.
#[derive(Clone)]
pub struct Secret(SecretString);

impl Secret {
    /// Hands out the value. Call only where it leaves the process, such as building connection options.
    #[must_use]
    pub fn expose(&self) -> &str {
        self.0.expose_secret()
    }
}

impl From<&str> for Secret {
    fn from(value: &str) -> Self {
        Self(SecretString::from(value))
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}

/// Where a secret comes from. This is what configuration files and environment variables contain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum SecretRef {
    /// Secret kept in an environment variable, e.g. `env:MY_TOKEN`.
    Env(String),
    /// Secret kept in a file, e.g. `file:/run/secrets/my_token`.
    File(PathBuf),
    /// Plain value, for development only. Written as the value itself, or `literal:VALUE` when it
    /// would otherwise look like a reference.
    Literal(Secret),
}

impl SecretRef {
    /// Reads the secret from its source.
    ///
    /// # Errors
    ///
    /// Returns [`SecretError`] when the source is missing, unreadable or empty.
    pub fn resolve(&self) -> Result<Secret, SecretError> {
        let secret = match self {
            Self::Env(name) => resolve_env(name)?,
            Self::File(path) => resolve_file(path)?,
            Self::Literal(secret) => secret.clone(),
        };
        if secret.expose().is_empty() {
            return Err(SecretError::Empty {
                reference: self.to_string(),
            });
        }
        Ok(secret)
    }
}

fn resolve_env(name: &str) -> Result<Secret, SecretError> {
    match std::env::var(name) {
        Ok(value) => {
            let value = Zeroizing::new(value);
            Ok(Secret::from(value.as_str()))
        }
        Err(std::env::VarError::NotPresent) => Err(SecretError::EnvNotSet {
            name: name.to_owned(),
        }),
        // The error carries the variable's value, so it is dropped rather than attached as a source.
        Err(std::env::VarError::NotUnicode(_)) => Err(SecretError::EnvNotUnicode {
            name: name.to_owned(),
        }),
    }
}

fn resolve_file(path: &PathBuf) -> Result<Secret, SecretError> {
    let bytes = Zeroizing::new(std::fs::read(path).map_err(|source| match source.kind() {
        io::ErrorKind::NotFound => SecretError::FileNotFound { path: path.clone() },
        io::ErrorKind::PermissionDenied => SecretError::FileAccessDenied { path: path.clone() },
        _ => SecretError::FileRead {
            path: path.clone(),
            source,
        },
    })?);
    let text = std::str::from_utf8(&bytes).map_err(|source| SecretError::FileNotUtf8 {
        path: path.clone(),
        source,
    })?;
    // Editors and Kubernetes Secret volumes commonly append a newline that is not part of the value.
    let text = text.strip_suffix('\n').unwrap_or(text);
    let text = text.strip_suffix('\r').unwrap_or(text);
    Ok(Secret::from(text))
}

impl TryFrom<String> for SecretRef {
    type Error = SecretError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = Zeroizing::new(value);
        if let Some(name) = value.strip_prefix(ENV_PREFIX) {
            if name.is_empty() {
                return Err(SecretError::InvalidReference {
                    reason: "`env:` needs a variable name",
                });
            }
            return Ok(Self::Env(name.to_owned()));
        }
        if let Some(path) = value.strip_prefix(FILE_PREFIX) {
            if path.is_empty() {
                return Err(SecretError::InvalidReference {
                    reason: "`file:` needs a path",
                });
            }
            return Ok(Self::File(PathBuf::from(path)));
        }
        let literal = value.strip_prefix(LITERAL_PREFIX).unwrap_or(&value);
        Ok(Self::Literal(Secret::from(literal)))
    }
}

impl fmt::Display for SecretRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Env(name) => write!(f, "{ENV_PREFIX}{name}"),
            Self::File(path) => write!(f, "{FILE_PREFIX}{}", path.display()),
            Self::Literal(_) => write!(f, "{LITERAL_PREFIX}[REDACTED]"),
        }
    }
}

impl From<SecretRef> for String {
    fn from(value: SecretRef) -> Self {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{SecretError, SecretRef};

    fn parse(value: &str) -> Result<SecretRef, SecretError> {
        SecretRef::try_from(value.to_owned())
    }

    #[test]
    fn parses_references_and_literals() -> Result<(), SecretError> {
        assert!(matches!(parse("env:DB_URL")?, SecretRef::Env(name) if name == "DB_URL"));
        assert!(matches!(parse("file:/run/secrets/db")?, SecretRef::File(_)));
        assert!(matches!(parse("hunter2")?, SecretRef::Literal(s) if s.expose() == "hunter2"));
        assert!(matches!(parse("literal:env:x")?, SecretRef::Literal(s) if s.expose() == "env:x"));
        assert!(matches!(
            parse("env:"),
            Err(SecretError::InvalidReference { .. })
        ));
        assert!(matches!(
            parse("file:"),
            Err(SecretError::InvalidReference { .. })
        ));
        Ok(())
    }

    #[test]
    fn display_never_shows_literal_values() -> Result<(), SecretError> {
        let printed = parse("hunter2")?.to_string();
        assert_eq!(printed, "literal:[REDACTED]");
        assert!(!format!("{:?}", parse("hunter2")?).contains("hunter2"));
        Ok(())
    }

    #[test]
    fn resolves_files_and_trims_one_newline() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!("magpie-secret-test-{}", std::process::id()));
        std::fs::write(&path, "s3cret\r\n")?;
        let resolved = SecretRef::File(path.clone()).resolve();
        std::fs::remove_file(&path)?;
        assert_eq!(resolved?.expose(), "s3cret");
        Ok(())
    }

    #[test]
    fn reports_missing_sources() {
        let missing = SecretRef::File(std::env::temp_dir().join("magpie-secret-test-missing"));
        assert!(matches!(
            missing.resolve(),
            Err(SecretError::FileNotFound { .. })
        ));
        let unset = SecretRef::Env("MAGPIE_SECRET_TEST_UNSET_VARIABLE".to_owned());
        assert!(matches!(
            unset.resolve(),
            Err(SecretError::EnvNotSet { .. })
        ));
    }

    #[test]
    fn rejects_empty_values() -> Result<(), SecretError> {
        assert!(matches!(
            parse("")?.resolve(),
            Err(SecretError::Empty { .. })
        ));
        Ok(())
    }
}
