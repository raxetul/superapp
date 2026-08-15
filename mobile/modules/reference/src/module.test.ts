import { describe, expect, it } from 'vitest';
import { isValidManifest } from '../../../sdk/src/manifest';
import { isCompatible } from '../../../sdk/src/version';
import { READ_PERMISSION, referenceModule } from './module';
import { ReferenceScreen } from './ReferenceScreen';

describe('TR-09-007 reference module (mobile)', () => {
  it('declares the read permission and a gated screen', () => {
    expect(referenceModule.permissions).toContain(READ_PERMISSION);
    expect(referenceModule.screens[0].requiredPermission).toBe(READ_PERMISSION);
  });

  it('declares an SDK version compatible with this SDK\'s own rule', () => {
    expect(isCompatible(referenceModule.sdkVersion)).toBe(true);
  });

  it('the screen wires the same component the module exports', () => {
    expect(referenceModule.screens[0].component).toBe(ReferenceScreen);
  });

  it('the screen component renders the expected element', () => {
    const element = ReferenceScreen();
    expect(element.props.testID).toBe('reference-screen');
    expect(element.props.children).toBe('hello from the reference module');
  });
});

describe('SDK manifest validator is reachable', () => {
  it('accepts a minimal valid manifest', () => {
    expect(isValidManifest({ name: 'reference', version: '1.0.0' })).toBe(true);
  });
});
