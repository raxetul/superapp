/** TR-08-004 / FR-08-002 — role-based navigation guard logic. */
import {
  ADMIN_ONLY_SCREENS,
  canAccessAdmin,
  canAccessScreen,
  resolveStack,
  visibleScreensForRole,
} from './guards';

describe('resolveStack (TR-08-004)', () => {
  it('routes by auth status', () => {
    expect(resolveStack('loading')).toBe('loading');
    expect(resolveStack('unauthenticated')).toBe('auth');
    expect(resolveStack('authenticated')).toBe('app');
  });
});

describe('admin guards (FR-08-002)', () => {
  it('grants admin areas only to the admin role', () => {
    expect(canAccessAdmin('admin')).toBe(true);
    expect(canAccessAdmin('Admin')).toBe(true);
    expect(canAccessAdmin('user')).toBe(false);
    expect(canAccessAdmin(null)).toBe(false);
  });

  it('blocks a regular user from every admin-only screen', () => {
    for (const screen of ADMIN_ONLY_SCREENS) {
      expect(canAccessScreen(screen, 'user')).toBe(false);
      expect(canAccessScreen(screen, 'admin')).toBe(true);
    }
  });

  it('always allows non-admin screens', () => {
    expect(canAccessScreen('Home', 'user')).toBe(true);
    expect(canAccessScreen('Home', null)).toBe(true);
  });

  it('filters visible screens by role', () => {
    expect(visibleScreensForRole('user')).toEqual(['Home']);
    expect(visibleScreensForRole('admin')).toEqual(['Home', 'Admin']);
  });
});
