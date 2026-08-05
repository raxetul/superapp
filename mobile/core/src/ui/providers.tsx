/**
 * App-wide UI providers: Tamagui theming + safe-area insets. Shared by the app
 * entry point and by component tests so rendered trees match production.
 */
import React from 'react';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { TamaguiProvider } from './tamagui';
import { tamaguiConfig } from '../../tamagui.config';

export interface UiProvidersProps {
  children: React.ReactNode;
  defaultTheme?: 'light' | 'dark';
}

export function UiProviders({ children, defaultTheme = 'light' }: UiProvidersProps) {
  return (
    <TamaguiProvider config={tamaguiConfig} defaultTheme={defaultTheme}>
      <SafeAreaProvider
        initialMetrics={{
          frame: { x: 0, y: 0, width: 0, height: 0 },
          insets: { top: 0, left: 0, right: 0, bottom: 0 },
        }}
      >
        {children}
      </SafeAreaProvider>
    </TamaguiProvider>
  );
}
