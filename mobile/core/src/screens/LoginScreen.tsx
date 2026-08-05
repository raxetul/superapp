/**
 * Login screen (FR-08-001 SSO login, FR-08-004 conditional self-registration).
 *
 * Logs in via Rauthy (OIDC SSO covering username/password inside the IdP).
 * The "Create account" affordance is shown only when the backend reports
 * `capabilities.self_registration_enabled`.
 */
import React, { useCallback, useEffect, useState } from 'react';
import { Button, H1, Paragraph, Spinner, YStack } from '@/ui/tamagui';
import { AuthApi, type Capabilities } from '@/api/endpoints';
import { useAuth } from '@/auth/AuthContext';

export interface LoginScreenProps {
  /** Invoked when the user opts into self-registration. */
  onRegister?: () => void;
  /** Overridable capabilities loader (defaults to the context API). */
  loadCapabilities?: () => Promise<Capabilities>;
}

export function LoginScreen({ onRegister, loadCapabilities }: LoginScreenProps) {
  const { login, error, api } = useAuth();
  const [capabilities, setCapabilities] = useState<Capabilities | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const loader = loadCapabilities ?? (() => new AuthApi(api).capabilities());
    loader()
      .then((caps) => {
        if (!cancelled) setCapabilities(caps);
      })
      .catch(() => {
        if (!cancelled) setCapabilities(null);
      });
    return () => {
      cancelled = true;
    };
  }, [api, loadCapabilities]);

  const handleLogin = useCallback(async () => {
    setBusy(true);
    try {
      await login();
    } catch {
      // error surfaced via context `error`
    } finally {
      setBusy(false);
    }
  }, [login]);

  return (
    <YStack flex={1} padding="$4" gap="$3" justifyContent="center" testID="login-screen">
      <H1>Welcome to SuperApp</H1>
      <Button testID="sso-login-button" disabled={busy} onPress={() => void handleLogin()}>
        {busy ? <Spinner /> : 'Sign in with SSO'}
      </Button>

      {capabilities?.self_registration_enabled ? (
        <Button testID="register-button" onPress={() => onRegister?.()}>
          Create an account
        </Button>
      ) : null}

      {error ? (
        <Paragraph testID="login-error" color="red">
          {error}
        </Paragraph>
      ) : null}
    </YStack>
  );
}

export default LoginScreen;
