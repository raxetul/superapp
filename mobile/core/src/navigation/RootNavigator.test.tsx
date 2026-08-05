/** TR-08-004 — role-based navigation guards drive which stack/screens mount. */
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react-native';
import { NavigationContainer } from '@react-navigation/native';
import { RootNavigator, appStackScreens } from './RootNavigator';
import { AuthProvider } from '@/auth/AuthContext';
import { UiProviders } from '@/ui/providers';
import { makeMeApiClientFactory, makeMemoryStore, testConfig } from '@/test-utils';
import type { Role } from '@/api/endpoints';
import type { StoredTokens } from '@/auth/tokenStorage';

const authed = { accessToken: 'a', refreshToken: 'r' } as StoredTokens;

async function renderNav(opts: { tokens?: StoredTokens | null; role?: Role }) {
  await render(
    <UiProviders>
      <AuthProvider
        config={testConfig}
        storage={makeMemoryStore(opts.tokens ?? null)}
        createApiClient={makeMeApiClientFactory(opts.role ?? 'user')}
      >
        <NavigationContainer>
          <RootNavigator />
        </NavigationContainer>
      </AuthProvider>
    </UiProviders>,
  );
}

describe('appStackScreens (TR-08-004)', () => {
  it('registers admin screens only for admins', () => {
    expect(appStackScreens(false)).toEqual(['Home']);
    expect(appStackScreens(true)).toEqual(['Home', 'Admin']);
  });
});

describe('RootNavigator (TR-08-004)', () => {
  it('shows the auth stack (login) when unauthenticated', async () => {
    await renderNav({ tokens: null });
    await waitFor(() => expect(screen.getByTestId('login-screen')).toBeTruthy());
  });

  it('shows the app home for an authenticated user', async () => {
    await renderNav({ tokens: authed, role: 'user' });
    await waitFor(() => expect(screen.getByTestId('home-screen')).toBeTruthy());
    expect(screen.queryByTestId('login-screen')).toBeNull();
  });

  it('shows the app home for an authenticated admin', async () => {
    await renderNav({ tokens: authed, role: 'admin' });
    await waitFor(() => expect(screen.getByTestId('home-screen')).toBeTruthy());
  });
});
