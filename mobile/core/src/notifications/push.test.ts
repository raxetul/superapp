/** TR-08-008 — background push: permission, token registration, deep-link routing. */
import * as Notifications from 'expo-notifications';
import {
  attachNotificationRouting,
  dataFromResponse,
  ensurePermissions,
  registerForPush,
  routeForNotification,
} from './push';

const mockNotifications = jest.mocked(Notifications);

describe('routeForNotification (TR-08-008 deep-link)', () => {
  it('routes to an explicit screen with params', () => {
    expect(routeForNotification({ screen: 'Admin', params: { tab: 'roles' } })).toEqual({
      screen: 'Admin',
      params: { tab: 'roles' },
    });
  });

  it('maps a known event type to its screen', () => {
    expect(routeForNotification({ type: 'admin.allowlist.updated' })).toEqual({ screen: 'Admin' });
    expect(routeForNotification({ type: 'user.role.changed' })).toEqual({ screen: 'Home' });
  });

  it('returns null for empty / unroutable payloads', () => {
    expect(routeForNotification(null)).toBeNull();
    expect(routeForNotification({})).toBeNull();
    expect(routeForNotification({ type: 'unknown.event' })).toBeNull();
  });
});

describe('permissions and registration', () => {
  it('registers and returns a push token when permission is granted', async () => {
    mockNotifications.getPermissionsAsync.mockResolvedValueOnce({
      status: 'granted',
      granted: true,
      canAskAgain: true,
    } as never);
    const reg = await registerForPush();
    expect(reg).toEqual({ token: 'ExponentPushToken[stub]', type: 'expo' });
  });

  it('requests permission when not yet granted', async () => {
    mockNotifications.getPermissionsAsync.mockResolvedValueOnce({
      status: 'undetermined',
      granted: false,
      canAskAgain: true,
    } as never);
    mockNotifications.requestPermissionsAsync.mockResolvedValueOnce({
      status: 'granted',
      granted: true,
      canAskAgain: true,
    } as never);
    expect(await ensurePermissions()).toBe(true);
    expect(mockNotifications.requestPermissionsAsync).toHaveBeenCalled();
  });

  it('returns null (no token) when permission is denied and cannot be re-asked', async () => {
    mockNotifications.getPermissionsAsync.mockResolvedValueOnce({
      status: 'denied',
      granted: false,
      canAskAgain: false,
    } as never);
    expect(await registerForPush()).toBeNull();
  });
});

describe('attachNotificationRouting', () => {
  it('navigates on a notification tap and returns an unsubscribe', () => {
    const remove = jest.fn();
    let handler: (r: unknown) => void = () => {};
    mockNotifications.addNotificationResponseReceivedListener.mockImplementationOnce(((
      cb: (r: unknown) => void,
    ) => {
      handler = cb;
      return { remove };
    }) as unknown as typeof Notifications.addNotificationResponseReceivedListener);

    const navigate = jest.fn();
    const unsubscribe = attachNotificationRouting(navigate);

    handler({ notification: { request: { content: { data: { screen: 'Home' } } } } });
    expect(navigate).toHaveBeenCalledWith({ screen: 'Home', params: undefined });

    unsubscribe();
    expect(remove).toHaveBeenCalled();
  });

  it('extracts the data payload from a response', () => {
    expect(
      dataFromResponse({
        notification: { request: { content: { data: { type: 'x' } } } },
      } as never),
    ).toEqual({ type: 'x' });
    expect(dataFromResponse(null)).toBeNull();
  });
});
