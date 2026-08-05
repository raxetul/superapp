/**
 * Root navigator with role-based guards (TR-08-004, FR-08-002).
 *
 * - Unauthenticated sessions see only the auth stack (login/register).
 * - Authenticated sessions see the app stack; admin-only screens are
 *   registered *only* for admins, so a regular user cannot navigate to them.
 *
 * The screen-selection decision is factored into pure, unit-tested helpers
 * ({@link resolveStack}, {@link appStackScreens}) so the guard behaviour is
 * verifiable without driving a live navigator on a device.
 */
import React from 'react';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { Spinner, YStack } from '@/ui/tamagui';
import { useAuth } from '@/auth/AuthContext';
import type { AppStackParamList, AuthStackParamList } from './types';
import { resolveStack } from './guards';
import { LoginScreen } from '@/screens/LoginScreen';
import { RegisterScreen } from '@/screens/RegisterScreen';
import { HomeScreen } from '@/screens/HomeScreen';
import { AdminScreen } from '@/screens/admin/AdminScreen';

const AuthStack = createNativeStackNavigator<AuthStackParamList>();
const AppStack = createNativeStackNavigator<AppStackParamList>();

/** Which app-stack screens to register for the given role. */
export function appStackScreens(isAdmin: boolean): Array<keyof AppStackParamList> {
  return isAdmin ? ['Home', 'Admin'] : ['Home'];
}

function LoadingView() {
  return (
    <YStack flex={1} alignItems="center" justifyContent="center" testID="loading-screen">
      <Spinner />
    </YStack>
  );
}

function AuthNavigator() {
  return (
    <AuthStack.Navigator screenOptions={{ headerShown: false }}>
      <AuthStack.Screen name="Login" component={LoginScreen} />
      <AuthStack.Screen name="Register" component={RegisterScreen} />
    </AuthStack.Navigator>
  );
}

function AppNavigator({ isAdmin }: { isAdmin: boolean }) {
  const screens = appStackScreens(isAdmin);
  return (
    <AppStack.Navigator>
      {screens.includes('Home') ? <AppStack.Screen name="Home" component={HomeScreen} /> : null}
      {screens.includes('Admin') ? <AppStack.Screen name="Admin" component={AdminScreen} /> : null}
    </AppStack.Navigator>
  );
}

export function RootNavigator() {
  const { status, isAdmin } = useAuth();
  const stack = resolveStack(status);

  if (stack === 'loading') return <LoadingView />;
  if (stack === 'auth') return <AuthNavigator />;
  return <AppNavigator isAdmin={isAdmin} />;
}

export default RootNavigator;
