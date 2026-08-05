/**
 * Global Jest setup. Native Expo modules are replaced with lightweight,
 * behaviour-preserving fakes so unit/component tests run in Node without a
 * device, simulator, Keychain/Keystore, or system browser. Individual tests
 * refine these via `jest.mocked(...)` when they need to assert specific calls.
 */

// React 19 requires this global so `act(...)` (used by Testing Library's
// render/renderHook) flushes effects instead of warning and no-op'ing.
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// In-memory secure store backing the expo-secure-store mock (Keychain/Keystore
// stand-in). Reset before every test to avoid cross-test leakage.
const mockSecureStore = new Map<string, string>();

jest.mock('expo-secure-store', () => ({
  setItemAsync: jest.fn(async (key: string, value: string) => {
    mockSecureStore.set(key, value);
  }),
  getItemAsync: jest.fn(async (key: string) =>
    mockSecureStore.has(key) ? mockSecureStore.get(key)! : null,
  ),
  deleteItemAsync: jest.fn(async (key: string) => {
    mockSecureStore.delete(key);
  }),
  isAvailableAsync: jest.fn(async () => true),
}));

jest.mock('expo-auth-session', () => ({
  makeRedirectUri: jest.fn((opts?: { scheme?: string; path?: string }) => {
    const scheme = opts?.scheme ?? 'superapp';
    const path = opts?.path ?? 'oauthredirect';
    return `${scheme}://${path}`;
  }),
  fetchDiscoveryAsync: jest.fn(async (issuer: string) => ({
    authorizationEndpoint: `${issuer}/authorize`,
    tokenEndpoint: `${issuer}/token`,
    revocationEndpoint: `${issuer}/revoke`,
    endSessionEndpoint: `${issuer}/logout`,
  })),
  exchangeCodeAsync: jest.fn(),
  refreshAsync: jest.fn(),
  revokeAsync: jest.fn(async () => true),
  AuthRequest: jest.fn(),
}));

jest.mock('expo-notifications', () => ({
  getPermissionsAsync: jest.fn(async () => ({ status: 'granted', granted: true, canAskAgain: true })),
  requestPermissionsAsync: jest.fn(async () => ({ status: 'granted', granted: true, canAskAgain: true })),
  getExpoPushTokenAsync: jest.fn(async () => ({ data: 'ExponentPushToken[stub]', type: 'expo' })),
  getDevicePushTokenAsync: jest.fn(async () => ({ data: 'device-stub', type: 'apns' })),
  setNotificationHandler: jest.fn(),
  addNotificationReceivedListener: jest.fn(() => ({ remove: jest.fn() })),
  addNotificationResponseReceivedListener: jest.fn(() => ({ remove: jest.fn() })),
  getLastNotificationResponseAsync: jest.fn(async () => null),
  setNotificationChannelAsync: jest.fn(async () => null),
  AndroidImportance: { MAX: 5, HIGH: 4, DEFAULT: 3 },
}));

jest.mock('expo-linking', () => ({
  createURL: jest.fn((path: string) => `superapp://${path}`),
  openURL: jest.fn(async () => true),
  addEventListener: jest.fn(() => ({ remove: jest.fn() })),
}));

jest.mock('expo-constants', () => ({
  __esModule: true,
  default: { expoConfig: { scheme: 'superapp', extra: {} } },
}));

beforeEach(() => {
  mockSecureStore.clear();
});
