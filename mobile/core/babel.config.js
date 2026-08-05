module.exports = function (api) {
  api.cache(true);

  const plugins = [
    [
      'module-resolver',
      {
        root: ['./'],
        alias: { '@': './src' },
        extensions: ['.ts', '.tsx', '.js', '.jsx', '.json'],
      },
    ],
  ];

  // The Tamagui optimizing compiler flattens/extracts styles for production
  // native/web builds. It is intentionally NOT run under Jest (NODE_ENV=test)
  // where components render at runtime through the standard React reconciler.
  if (process.env.NODE_ENV !== 'test') {
    plugins.push([
      '@tamagui/babel-plugin',
      {
        components: ['tamagui'],
        config: './tamagui.config.ts',
        logTimings: true,
        disableExtraction: process.env.NODE_ENV === 'development',
      },
    ]);
  }

  return {
    presets: ['babel-preset-expo'],
    plugins,
  };
};
