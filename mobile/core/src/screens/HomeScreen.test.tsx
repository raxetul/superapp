/** TR-08-001 — baseline authenticated screen renders with Tamagui. */
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react-native';
import { HomeScreen } from './HomeScreen';
import { AuthProvider } from '@/auth/AuthContext';
import { UiProviders } from '@/ui/providers';
import { makeMeApiClientFactory, makeMemoryStore, testConfig } from '@/test-utils';
import type { Role } from '@/api/endpoints';
import type { StoredTokens } from '@/auth/tokenStorage';

async function renderHome(role: Role) {
  await render(
    <UiProviders>
      <AuthProvider
        config={testConfig}
        storage={makeMemoryStore({ accessToken: 'a', refreshToken: 'r' } as StoredTokens)}
        createApiClient={makeMeApiClientFactory(role)}
      >
        <HomeScreen />
      </AuthProvider>
    </UiProviders>,
  );
}

describe('HomeScreen (TR-08-001)', () => {
  it('renders the baseline screen with the signed-in identity', async () => {
    await renderHome('user');
    expect(screen.getByTestId('home-screen')).toBeTruthy();
    expect(screen.getByTestId('logout-button')).toBeTruthy();
    await waitFor(() =>
      expect(screen.getByTestId('home-greeting')).toHaveTextContent('Signed in as Ada Lovelace'),
    );
    expect(screen.getByTestId('home-role')).toHaveTextContent('Role: User');
  });

  it('reflects the admin role', async () => {
    await renderHome('admin');
    await waitFor(() => expect(screen.getByTestId('home-role')).toHaveTextContent('Role: Admin'));
  });
});
