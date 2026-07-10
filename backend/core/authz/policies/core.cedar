// SuperApp core authorization policies (TR-04-006).
//
// These files are loaded from disk at startup, so editing them changes
// decisions without recompiling the backend. Policy ids are stable and are
// surfaced in the authorization audit log (TR-04-008).

// Administrators may perform any action on any resource.
@id("admin-full-access")
permit(
  principal in Role::"admin",
  action,
  resource
);

// Any user may read their own profile.
@id("self-profile-read")
permit(
  principal,
  action == Action::"profile.read",
  resource
)
when { principal == resource };
