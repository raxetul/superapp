/**
 * Minimal Tamagui UI kit built on `@tamagui/core` primitives.
 *
 * The kitchen-sink `tamagui` package pulls in popper/menu components that need
 * extra native peers; the core primitives cover the baseline screens (layout,
 * text, buttons, inputs) with the full Tamagui theming/token pipeline and no
 * native-only dependencies — so components render both on-device and under
 * Jest.
 */
import React from 'react';
import { ActivityIndicator, ScrollView as RNScrollView, TextInput, TouchableOpacity } from 'react-native';
import { styled, Text as TamaguiText, View } from '@tamagui/core';

export { TamaguiProvider } from '@tamagui/core';

export const YStack = styled(View, { name: 'YStack', flexDirection: 'column' });
export const XStack = styled(View, { name: 'XStack', flexDirection: 'row' });

export const Paragraph = styled(TamaguiText, { name: 'Paragraph', fontSize: 15 });
export const H1 = styled(TamaguiText, { name: 'H1', fontSize: 30, fontWeight: '700' });
export const H2 = styled(TamaguiText, { name: 'H2', fontSize: 22, fontWeight: '700' });

const ButtonFrame = styled(View, {
  name: 'ButtonFrame',
  backgroundColor: '#1f6feb',
  paddingVertical: 12,
  paddingHorizontal: 16,
  borderRadius: 8,
  alignItems: 'center',
  justifyContent: 'center',
});

const ButtonLabel = styled(TamaguiText, { name: 'ButtonLabel', color: 'white', fontWeight: '600' });

export interface ButtonProps {
  children?: React.ReactNode;
  onPress?: () => void;
  disabled?: boolean;
  testID?: string;
}

export function Button({ children, onPress, disabled, testID }: ButtonProps) {
  return (
    <TouchableOpacity testID={testID} disabled={disabled} onPress={onPress} accessibilityRole="button">
      <ButtonFrame opacity={disabled ? 0.6 : 1}>
        {typeof children === 'string' ? <ButtonLabel>{children}</ButtonLabel> : children}
      </ButtonFrame>
    </TouchableOpacity>
  );
}

export const Separator = styled(View, {
  name: 'Separator',
  height: 1,
  backgroundColor: '#d0d7de',
  alignSelf: 'stretch',
});

export interface InputProps {
  value?: string;
  onChangeText?: (text: string) => void;
  placeholder?: string;
  autoCapitalize?: 'none' | 'sentences' | 'words' | 'characters';
  testID?: string;
  flex?: number;
}

export function Input({ value, onChangeText, placeholder, autoCapitalize, testID, flex }: InputProps) {
  return (
    <TextInput
      testID={testID}
      value={value}
      onChangeText={onChangeText}
      placeholder={placeholder}
      autoCapitalize={autoCapitalize}
      style={{
        flex,
        borderWidth: 1,
        borderColor: '#d0d7de',
        borderRadius: 8,
        paddingHorizontal: 12,
        paddingVertical: 10,
      }}
    />
  );
}

export function Spinner({ testID }: { testID?: string }) {
  return <ActivityIndicator testID={testID} />;
}

export function ScrollView({ children, testID }: { children?: React.ReactNode; testID?: string }) {
  return <RNScrollView testID={testID}>{children}</RNScrollView>;
}
