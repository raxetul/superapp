/**
 * Admin management screen (FR-08-003): allow-list by email and manage roles,
 * at parity with the web admin surface. Reachable only by admins (TR-08-004).
 */
import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Button, H2, Input, Paragraph, ScrollView, Separator, XStack, YStack } from '@/ui/tamagui';
import { ApiError } from '@/api/client';
import { AdminApi, type AllowlistEntry, type Role } from '@/api/endpoints';
import { useAuth } from '@/auth/AuthContext';

export function AdminScreen() {
  const { api } = useAuth();
  const adminApi = useMemo(() => new AdminApi(api), [api]);

  const [entries, setEntries] = useState<AllowlistEntry[]>([]);
  const [newEmail, setNewEmail] = useState('');
  const [roleEmail, setRoleEmail] = useState('');
  const [error, setError] = useState<string | null>(null);

  const surface = useCallback((e: unknown) => {
    setError(e instanceof ApiError ? e.problem.detail ?? e.problem.title : 'Request failed');
  }, []);

  const refresh = useCallback(async () => {
    try {
      setEntries(await adminApi.listAllowlist());
    } catch (e) {
      surface(e);
    }
  }, [adminApi, surface]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const addEntry = useCallback(async () => {
    if (!newEmail) return;
    setError(null);
    try {
      await adminApi.addAllowlist(newEmail);
      setNewEmail('');
      await refresh();
    } catch (e) {
      surface(e);
    }
  }, [adminApi, newEmail, refresh, surface]);

  const setRole = useCallback(
    async (role: Role) => {
      if (!roleEmail) return;
      setError(null);
      try {
        await adminApi.setUserRole(roleEmail, role);
      } catch (e) {
        surface(e);
      }
    },
    [adminApi, roleEmail, surface],
  );

  return (
    <ScrollView testID="admin-screen">
      <YStack padding="$4" gap="$4">
        <H2>Admin</H2>

        <YStack gap="$2" testID="allowlist-section">
          <Paragraph fontWeight="700">Allow-list</Paragraph>
          <XStack gap="$2">
            <Input
              flex={1}
              testID="allowlist-email-input"
              placeholder="email@company.com"
              autoCapitalize="none"
              value={newEmail}
              onChangeText={setNewEmail}
            />
            <Button testID="allowlist-add-button" onPress={() => void addEntry()}>
              Add
            </Button>
          </XStack>
          <YStack testID="allowlist-entries" gap="$1">
            {entries.map((entry) => (
              <Paragraph key={entry.email} testID={`allowlist-entry-${entry.email}`}>
                {entry.email}
                {entry.role ? ` — ${entry.role}` : ''}
              </Paragraph>
            ))}
          </YStack>
        </YStack>

        <Separator />

        <YStack gap="$2" testID="roles-section">
          <Paragraph fontWeight="700">Manage roles</Paragraph>
          <Input
            testID="role-email-input"
            placeholder="email@company.com"
            autoCapitalize="none"
            value={roleEmail}
            onChangeText={setRoleEmail}
          />
          <XStack gap="$2">
            <Button testID="make-admin-button" onPress={() => void setRole('admin')}>
              Make admin
            </Button>
            <Button testID="make-user-button" onPress={() => void setRole('user')}>
              Make user
            </Button>
          </XStack>
        </YStack>

        {error ? (
          <Paragraph testID="admin-error" color="red">
            {error}
          </Paragraph>
        ) : null}
      </YStack>
    </ScrollView>
  );
}

export default AdminScreen;
