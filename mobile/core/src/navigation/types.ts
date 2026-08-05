/** React Navigation param lists for the app's stacks. */

export type AuthStackParamList = {
  Login: undefined;
  Register: undefined;
};

export type AppStackParamList = {
  Home: undefined;
  Admin: undefined;
  /** Dynamically-registered module screens are routed by id. */
  Module: { moduleId: string; screen: string };
};
