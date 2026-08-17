//! SDK version compatibility (TR-09-005).
//!
//! A module built with a Rust/TypeScript module SDK may optionally report the
//! SDK version it was built against (`GET /sdk` on the module's service
//! contract, TR-09-001). The core accepts a module iff its reported SDK major
//! version is one this core release supports — a module reporting no version
//! at all (pre-P9 / non-SDK-authored, e.g. the P5 test fixtures) is treated as
//! compatible for backward compatibility.
//!
//! The same major-version rule is implemented independently by each SDK
//! (`backend/sdk`, `frontend/sdk`, `mobile/sdk`) so a module author can check
//! compatibility before shipping; this module is the core-side enforcement
//! point exercised at [`super::registry::ModuleRegistry::load`].

/// The SDK major version(s) this core release accepts.
pub const SUPPORTED_SDK_MAJOR: u32 = 1;

/// Why a reported SDK version was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompatError {
    #[error("module SDK version `{0}` is not a valid semver-style version")]
    Unparseable(String),
    #[error(
        "module SDK major version {reported} is incompatible with core-supported major {supported}"
    )]
    Incompatible { reported: u32, supported: u32 },
}

/// Parse the leading `MAJOR` component out of a `MAJOR.MINOR.PATCH`-style
/// version string.
///
/// # Errors
/// [`CompatError::Unparseable`] if the leading component is not a number.
pub fn major_version(version: &str) -> Result<u32, CompatError> {
    version
        .split('.')
        .next()
        .unwrap_or(version)
        .trim()
        .parse::<u32>()
        .map_err(|_| CompatError::Unparseable(version.to_string()))
}

/// Check `reported` (a module's declared SDK version) against
/// [`SUPPORTED_SDK_MAJOR`].
///
/// # Errors
/// [`CompatError`] if unparseable or on a different major version.
pub fn check_compatible(reported: &str) -> Result<(), CompatError> {
    let major = major_version(reported)?;
    if major == SUPPORTED_SDK_MAJOR {
        Ok(())
    } else {
        Err(CompatError::Incompatible {
            reported: major,
            supported: SUPPORTED_SDK_MAJOR,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_major_is_compatible() {
        assert!(check_compatible("1.0.0").is_ok());
        assert!(check_compatible("1.9.3").is_ok());
    }

    #[test]
    fn different_major_is_incompatible() {
        assert_eq!(
            check_compatible("2.0.0"),
            Err(CompatError::Incompatible {
                reported: 2,
                supported: 1
            })
        );
    }

    #[test]
    fn unparseable_version_is_rejected() {
        assert_eq!(
            check_compatible("not-a-version"),
            Err(CompatError::Unparseable("not-a-version".to_string()))
        );
    }
}
