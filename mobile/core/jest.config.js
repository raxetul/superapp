/** Jest configuration for the SuperApp mobile core (Expo). */
module.exports = {
  preset: 'jest-expo',
  setupFilesAfterEnv: ['<rootDir>/jest.setup.ts'],
  moduleNameMapper: {
    '^@/(.*)$': '<rootDir>/src/$1',
  },
  // jest-expo ships a broad allow-list; we extend it so Tamagui and React
  // Navigation ESM packages are transformed by Babel instead of being treated
  // as pre-compiled CommonJS.
  transformIgnorePatterns: [
    'node_modules/(?!((jest-)?react-native|@react-native(-community)?|expo(nent)?|@expo(nent)?/.*|@expo-google-fonts/.*|react-navigation|@react-navigation/.*|@unimodules/.*|unimodules|sentry-expo|native-base|react-native-svg|tamagui|@tamagui/.*|react-native-css-interop|@legendapp/.*))',
  ],
  collectCoverageFrom: ['src/**/*.{ts,tsx}', '!src/**/*.d.ts'],
  // TR-10-002: critical-path coverage gate (80%), enforced in CI via
  // `npm run test:coverage`.
  coverageThreshold: {
    global: {
      lines: 80,
      functions: 80,
      branches: 80,
      statements: 80,
    },
  },
  clearMocks: true,
};
