/**
 * Background push notifications (TR-08-008).
 *
 * SSE (TR-08-006) only runs while the app is in the foreground, so background
 * delivery uses OS push (APNs on iOS, FCM on Android) via `expo-notifications`.
 * This module: requests permission, obtains the push token to register with
 * the backend, and maps a tapped notification's data payload to an in-app
 * route for deep-linking.
 *
 * DEFERRED / cross-phase dependency: the backend has no push-send capability
 * yet (P6 delivers only foreground SSE). Wiring the obtained token to a backend
 * device-registration endpoint and having the backend fan events out to
 * APNs/FCM is documented in the phase doc as a carry-forward. This module
 * implements the entire client half.
 */
import * as Notifications from 'expo-notifications';

export interface PushRegistration {
  token: string;
  type: string;
}

export interface NotificationRoute {
  screen: string;
  params?: Record<string, unknown>;
}

/** Well-known backend event types → target screen for deep linking. */
export const EVENT_TYPE_ROUTES: Record<string, string> = {
  'admin.allowlist.updated': 'Admin',
  'admin.role.changed': 'Admin',
  'user.role.changed': 'Home',
};

/** Ensure notification permission, requesting it once if askable. */
export async function ensurePermissions(): Promise<boolean> {
  const current = await Notifications.getPermissionsAsync();
  if (current.granted || current.status === 'granted') return true;
  if (current.canAskAgain === false) return false;
  const requested = await Notifications.requestPermissionsAsync();
  return requested.granted || requested.status === 'granted';
}

/**
 * Register for push and return the token to hand to the backend. Returns null
 * if permission was denied.
 */
export async function registerForPush(): Promise<PushRegistration | null> {
  const granted = await ensurePermissions();
  if (!granted) return null;
  const token = await Notifications.getExpoPushTokenAsync();
  return { token: token.data, type: token.type ?? 'expo' };
}

/**
 * Map a notification data payload to an in-app route. Precedence:
 * 1. explicit `screen` (+ optional `params`) in the payload;
 * 2. a well-known `type` mapped via {@link EVENT_TYPE_ROUTES}.
 * Returns null when nothing routable is present.
 */
export function routeForNotification(
  data: Record<string, unknown> | null | undefined,
): NotificationRoute | null {
  if (!data) return null;

  if (typeof data.screen === 'string' && data.screen.length > 0) {
    const params =
      typeof data.params === 'object' && data.params !== null
        ? (data.params as Record<string, unknown>)
        : undefined;
    return { screen: data.screen, params };
  }

  if (typeof data.type === 'string' && data.type in EVENT_TYPE_ROUTES) {
    return { screen: EVENT_TYPE_ROUTES[data.type] };
  }

  return null;
}

/** Extract the data payload from a notification response object. */
export function dataFromResponse(
  response: Notifications.NotificationResponse | null | undefined,
): Record<string, unknown> | null {
  const data = response?.notification?.request?.content?.data;
  return (data as Record<string, unknown> | undefined) ?? null;
}

/**
 * Subscribe to notification taps and route them. Returns an unsubscribe fn.
 * Also handles a cold start (app launched by tapping a notification).
 */
export function attachNotificationRouting(navigate: (route: NotificationRoute) => void): () => void {
  const subscription = Notifications.addNotificationResponseReceivedListener((response) => {
    const route = routeForNotification(dataFromResponse(response));
    if (route) navigate(route);
  });

  void Notifications.getLastNotificationResponseAsync().then((response) => {
    const route = routeForNotification(dataFromResponse(response));
    if (route) navigate(route);
  });

  return () => subscription.remove();
}
