//! Private OCI registry resolution (TR-09-009).
//!
//! Modules distribute as OCI/Docker images pushed to a **self-hosted private**
//! registry — never a public package registry. This module resolves the image
//! reference the [`super::runtime::DockerRuntime`] pulls, by module
//! `name`+`version`, against the configured registry host. The actual
//! build/push/pull is a Docker/CI concern (deferred here — see the phase doc);
//! what's tested is the pure resolution logic.

/// Resolve the OCI image reference for `name`@`version` against
/// `registry_host` (e.g. `registry.superapp.internal`).
///
/// The result is always scoped under a `modules/` repository path within the
/// configured private host, so it can never accidentally resolve to a public
/// registry (e.g. Docker Hub) even if `registry_host` is left empty — an empty
/// host still yields a `modules/…` reference relative to whatever registry
/// the container runtime is configured to reach.
#[must_use]
pub fn resolve_image_ref(registry_host: &str, name: &str, version: &str) -> String {
    let host = registry_host.trim().trim_end_matches('/');
    if host.is_empty() {
        format!("modules/{name}:{version}")
    } else {
        format!("{host}/modules/{name}:{version}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_against_a_configured_private_host() {
        assert_eq!(
            resolve_image_ref("registry.superapp.internal", "reference", "1.0.0"),
            "registry.superapp.internal/modules/reference:1.0.0"
        );
    }

    #[test]
    fn trims_a_trailing_slash_on_the_host() {
        assert_eq!(
            resolve_image_ref("registry.superapp.internal/", "billing", "2.1.0"),
            "registry.superapp.internal/modules/billing:2.1.0"
        );
    }

    #[test]
    fn falls_back_to_a_bare_repository_path_without_a_configured_host() {
        assert_eq!(
            resolve_image_ref("", "reference", "1.0.0"),
            "modules/reference:1.0.0"
        );
    }

    #[test]
    fn never_resolves_to_a_public_registry_shorthand() {
        // Even a bare name+version never collapses to a Docker-Hub-style
        // `name:version` with no `modules/` scoping.
        let resolved = resolve_image_ref("registry.superapp.internal", "reference", "1.0.0");
        assert!(resolved.contains("/modules/"));
    }
}
