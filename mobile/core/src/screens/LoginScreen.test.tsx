/** FR-08-004 (conditional self-registration) + FR-08-001 (SSO login). */
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { LoginScreen } from './LoginScreen';
import { AuthProvider } from '@/auth/AuthContext';
import { UiProviders } from '@/ui/providers';
import { makeFakeOidc, makeMeApiClientFactory, makeMemoryStore, testConfig } from '@/test-utils';
import type { Capabilities } from '@/api/endpoints';

function renderLogin(opts: {
  capabilities: Capabilities;
  oidc?: ReturnType<typeof makeFakeOidc>;
  onRegister?: () => void;
}) {
  const oidc = opts.oidc ?? makeFakeOidc();
  return render(
    <UiProviders>
      <AuthProvider
        config={testConfig}
        oidcClient={oidc}
        storage={makeMemoryStore()}
        createApiClient={makeMeApiClientFactory('user')}
      >
        <LoginScreen loadCapabilities={async () => opts.capabilities} onRegister={opts.onRegister} />
      </AuthProvider>
    </UiProviders>,
  );
}

describe('LoginScreen', () => {
  it('always offers SSO sign-in', async () => {
    await renderLogin({ capabilities: { self_registration_enabled: false, oidc_configured: true } });
    expect(screen.getByTestId('sso-login-button')).toBeTruthy();
  });

  it('shows the self-registration option only when enabled (FR-08-004)', async () => {
    await renderLogin({ capabilities: { self_registration_enabled: true, oidc_configured: true } });
    await waitFor(() => expect(screen.getByTestId('register-button')).toBeTruthy());
  });

  it('hides the self-registration option when disabled (FR-08-004)', async () => {
    await renderLogin({ capabilities: { self_registration_enabled: false, oidc_configured: true } });
    // Give the capabilities effect a chance to resolve, then assert absence.
    await waitFor(() => expect(screen.getByTestId('sso-login-button')).toBeTruthy());
    expect(screen.queryByTestId('register-button')).toBeNull();
  });

  it('starts the OIDC login on SSO press (FR-08-001)', async () => {
    const oidc = makeFakeOidc();
    const loginSpy = jest.spyOn(oidc, 'login');
    await renderLogin({
      capabilities: { self_registration_enabled: false, oidc_configured: true },
      oidc,
    });
    await fireEvent.press(screen.getByTestId('sso-login-button'));
    await waitFor(() => expect(loginSpy).toHaveBeenCalled());
  });
});
