import { defaultConfig } from '@tamagui/config/v4';
import { createTamagui } from '@tamagui/core';

/**
 * App-wide Tamagui configuration. Built on the stock v4 preset so the baseline
 * screens render with a consistent theme/token set on iOS and Android.
 *
 * `defaultConfig` is cast to the `createTamagui` input shape: the v4 preset's
 * `animations` driver is structurally a superset of `AnimationsConfig`, and the
 * ambient module augmentation that would otherwise reconcile the two creates a
 * self-referential type cycle under TypeScript 6. The runtime object is exactly
 * the stock preset.
 */
export const tamaguiConfig = createTamagui(
  defaultConfig as Parameters<typeof createTamagui>[0],
);

export type AppTamaguiConfig = typeof tamaguiConfig;

export default tamaguiConfig;
