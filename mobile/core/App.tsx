import React from 'react';
import { StatusBar } from 'expo-status-bar';
import { NavigationContainer } from '@react-navigation/native';
import { UiProviders } from '@/ui/providers';
import { AuthProvider } from '@/auth/AuthContext';
import { RootNavigator } from '@/navigation/RootNavigator';

/**
 * App entry (TR-08-001). Composition root wiring UI theming, auth session,
 * and role-based navigation. Configuration is read from `EXPO_PUBLIC_*` env
 * vars by {@link AuthProvider} → a missing required var fails loudly at boot
 * (TR-08-007).
 */
export default function App() {
  return (
    <UiProviders>
      <AuthProvider>
        <NavigationContainer>
          <RootNavigator />
        </NavigationContainer>
        <StatusBar style="auto" />
      </AuthProvider>
    </UiProviders>
  );
}
