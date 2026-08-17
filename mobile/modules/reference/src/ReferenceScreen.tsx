/**
 * The reference module's mobile screen (TR-09-007). Uses a local functional
 * element rather than a `react-native` `Text` import — this package stays
 * dependency-light (no `react-native`/Expo needed to unit-test module
 * wiring), matching how `mobile/core`'s own screen tests never render real
 * RN primitives either (see `mobile/core/src/modules/registry.test.ts`).
 */
import * as React from 'react';

interface LabelProps {
  testID?: string;
  children?: React.ReactNode;
}

function Label(props: LabelProps): null {
  void props;
  return null;
}

export function ReferenceScreen(): React.ReactElement<LabelProps> {
  return React.createElement(Label, { testID: 'reference-screen' }, 'hello from the reference module');
}
