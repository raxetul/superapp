//! SDK version + compatibility (TR-09-005).
//!
//! [`SDK_VERSION`] is what [`crate::server::ModuleServer`] reports on `GET
//! /sdk`. [`is_compatible_with_core`] implements the same major-version rule
//! the core enforces at load time (`backend/core/src/modules/compat.rs`) so a
//! module author can check compatibility before ever registering.

/// This SDK's version. Bump the major component on a breaking contract
/// change (new required endpoint, changed manifest shape, …).
pub const SDK_VERSION: &str = "1.0.0";

/// Why a core's supported-major range rejected this SDK version.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompatError {
    #[error("SDK version `{0}` is not a valid semver-style version")]
    Unparseable(String),
    #[error("SDK major version {sdk} is incompatible with core-supported major {core}")]
    Incompatible { sdk: u32, core: u32 },
}

fn major_version(version: &str) -> Result<u32, CompatError> {
    version
        .split('.')
        .next()
        .unwrap_or(version)
        .trim()
        .parse::<u32>()
        .map_err(|_| CompatError::Unparseable(version.to_string()))
}

/// Check this SDK's version against a core's `supported_major` (as reported
/// out-of-band, e.g. in module-author docs or a future `/capabilities`
/// endpoint).
///
/// # Errors
/// [`CompatError`] on a differing major version.
pub fn is_compatible_with_core(supported_major: u32) -> Result<(), CompatError> {
    let major = major_version(SDK_VERSION)?;
    if major == supported_major {
        Ok(())
    } else {
        Err(CompatError::Incompatible {
            sdk: major,
            core: supported_major,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_version_is_semver_shaped() {
        assert_eq!(SDK_VERSION.split('.').count(), 3);
    }

    #[test]
    fn compatible_with_matching_major() {
        assert!(is_compatible_with_core(1).is_ok());
    }

    #[test]
    fn incompatible_with_a_different_major() {
        assert_eq!(
            is_compatible_with_core(2),
            Err(CompatError::Incompatible { sdk: 1, core: 2 })
        );
    }
}
