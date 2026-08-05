/**
 * Self-registration screen (FR-08-004). Only reachable when the backend
 * reports `self_registration_enabled`; registration itself completes in the
 * Rauthy-hosted flow, so this screen kicks off the same OIDC journey.
 */
import React from 'react';
import { Button, H1, Paragraph, YStack } from '@/ui/tamagui';
import { useAuth } from '@/auth/AuthContext';

export function RegisterScreen() {
  const { login } = useAuth();
  return (
    <YStack flex={1} padding="$4" gap="$3" justifyContent="center" testID="register-screen">
      <H1>Create your account</H1>
      <Paragraph>Registration is completed in the SuperApp identity provider.</Paragraph>
      <Button testID="register-continue-button" onPress={() => void login()}>
        Continue
      </Button>
    </YStack>
  );
}

export default RegisterScreen;
