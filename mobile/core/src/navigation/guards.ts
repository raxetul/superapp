/**
 * Pure navigation-guard logic (TR-08-004, FR-08-002).
 *
 * Kept free of React so the access rules are exhaustively unit-testable:
 * which stack a session sees, and whether a role may reach a given screen.
 */
import { isAdminRole, type Role } from '@/api/endpoints';
import type { AuthStatus } from '@/auth/AuthContext';

export type NavStack = 'loading' | 'auth' | 'app';

/** App screens that require the admin role. */
export const ADMIN_ONLY_SCREENS = ['Admin'] as const;
export type AppScreen = 'Home' | (typeof ADMIN_ONLY_SCREENS)[number];

/** Which top-level navigator a session should see. */
export function resolveStack(status: AuthStatus): NavStack {
  switch (status) {
    case 'loading':
      return 'loading';
    case 'authenticated':
      return 'app';
    case 'unauthenticated':
    default:
      return 'auth';
  }
}

/** Admin-only areas require the admin role. */
export function canAccessAdmin(role: Role | null | undefined): boolean {
  return isAdminRole(role);
}

/** Can a user with `role` reach `screen`? */
export function canAccessScreen(screen: AppScreen, role: Role | null | undefined): boolean {
  if ((ADMIN_ONLY_SCREENS as readonly string[]).includes(screen)) {
    return canAccessAdmin(role);
  }
  return true;
}

/** The app screens visible to `role` (admin areas filtered out for non-admins). */
export function visibleScreensForRole(role: Role | null | undefined): AppScreen[] {
  const all: AppScreen[] = ['Home', ...ADMIN_ONLY_SCREENS];
  return all.filter((screen) => canAccessScreen(screen, role));
}
