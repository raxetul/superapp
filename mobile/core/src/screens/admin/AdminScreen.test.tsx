/** FR-08-003 — admin management: allow-list by email and manage roles. */
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import { AdminScreen } from './AdminScreen';
import { ApiClient, type TokenProvider } from '@/api/client';
import { AuthProvider } from '@/auth/AuthContext';
import { UiProviders } from '@/ui/providers';
import { makeMemoryStore, testConfig } from '@/test-utils';
import type { StoredTokens } from '@/auth/tokenStorage';

async function renderAdmin() {
  const calls: Array<{ url: string; method: string; body?: unknown }> = [];
  const fetchImpl = (async (url: string, init: RequestInit) => {
    calls.push({
      url,
      method: init.method ?? 'GET',
      body: init.body ? JSON.parse(init.body as string) : undefined,
    });
    const data =
      typeof url === 'string' && url.endsWith('/admin/allowlist') && (init.method ?? 'GET') === 'GET'
        ? [{ email: 'existing@company.com', role: 'user' }]
        : {};
    return new Response(JSON.stringify({ success: true, data }), {
      status: 200,
      headers: { 'content-type': 'application/json' },
    });
  }) as unknown as typeof fetch;

  const createApiClient = (tp: TokenProvider) =>
    new ApiClient({ baseUrl: testConfig.apiBaseUrl, tokenProvider: tp, fetchImpl });

  await render(
    <UiProviders>
      <AuthProvider
        config={testConfig}
        storage={makeMemoryStore({ accessToken: 'a' } as StoredTokens)}
        createApiClient={createApiClient}
      >
        <AdminScreen />
      </AuthProvider>
    </UiProviders>,
  );
  return { calls };
}

describe('AdminScreen (FR-08-003)', () => {
  it('loads and lists existing allow-list entries', async () => {
    await renderAdmin();
    await waitFor(() =>
      expect(screen.getByTestId('allowlist-entry-existing@company.com')).toBeTruthy(),
    );
  });

  it('adds an allow-list entry by email', async () => {
    const { calls } = await renderAdmin();
    await waitFor(() => expect(screen.getByTestId('allowlist-add-button')).toBeTruthy());

    await fireEvent.changeText(screen.getByTestId('allowlist-email-input'), 'new@company.com');
    await fireEvent.press(screen.getByTestId('allowlist-add-button'));

    await waitFor(() => {
      const post = calls.find((c) => c.method === 'POST' && c.url.endsWith('/admin/allowlist'));
      expect(post?.body).toMatchObject({ email: 'new@company.com' });
    });
  });

  it('updates a user role', async () => {
    const { calls } = await renderAdmin();
    await waitFor(() => expect(screen.getByTestId('role-email-input')).toBeTruthy());

    await fireEvent.changeText(screen.getByTestId('role-email-input'), 'someone@company.com');
    await fireEvent.press(screen.getByTestId('make-admin-button'));

    await waitFor(() => {
      const put = calls.find((c) => c.method === 'PUT' && c.url.endsWith('/admin/users/role'));
      expect(put?.body).toEqual({ email: 'someone@company.com', role: 'admin' });
    });
  });
});
