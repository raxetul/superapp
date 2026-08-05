/**
 * Baseline authenticated screen (TR-08-001), rendered with Tamagui primitives.
 * Shows the signed-in identity and role, plus a logout affordance (FR-08-001).
 */
import React from 'react';
import { Button, H2, Paragraph, YStack } from '@/ui/tamagui';
import { useAuth } from '@/auth/AuthContext';

export function HomeScreen() {
  const { user, logout, isAdmin } = useAuth();

  return (
    <YStack flex={1} padding="$4" gap="$3" testID="home-screen">
      <H2>SuperApp</H2>
      <Paragraph testID="home-greeting">
        Signed in as {user?.name ?? user?.email ?? 'unknown'}
      </Paragraph>
      <Paragraph testID="home-role">Role: {isAdmin ? 'Admin' : 'User'}</Paragraph>
      <Button testID="logout-button" onPress={() => void logout()}>
        Log out
      </Button>
    </YStack>
  );
}

export default HomeScreen;
