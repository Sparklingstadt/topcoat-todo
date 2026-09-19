import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/playwright',
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  workers: 2,
  timeout: 30_000,
  expect: {
    timeout: 5_000,
    toHaveScreenshot: {
      animations: 'disabled',
      caret: 'hide',
      scale: 'css',
      maxDiffPixels: 0,
      threshold: 0.1,
    },
  },
  // Missing baselines must fail ordinary runs, including outside CI.
  updateSnapshots: 'none',
  snapshotPathTemplate: `{testDir}/__screenshots__/${process.platform}-${process.arch}/{projectName}/{arg}{ext}`,
  outputDir: './test-results',
  reporter: [
    ['list'],
    ['html', { outputFolder: 'playwright-report', open: 'never' }],
    ['json', { outputFile: 'test-results/results.json' }],
  ],
  use: {
    browserName: 'chromium',
    javaScriptEnabled: false,
    locale: 'ja-JP',
    timezoneId: 'Asia/Tokyo',
    colorScheme: 'light',
    reducedMotion: 'reduce',
    deviceScaleFactor: 1,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    video: 'retain-on-failure',
    serviceWorkers: 'block',
  },
  projects: [
    { name: 'e2e-desktop', testMatch: '**/*.e2e.spec.ts', use: { viewport: { width: 1280, height: 900 } } },
    { name: 'e2e-mobile', testMatch: '**/*.e2e.spec.ts', use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } },
    { name: 'visual-desktop', testMatch: '**/*.visual.spec.ts', use: { viewport: { width: 1280, height: 900 } } },
    { name: 'visual-mobile', testMatch: '**/*.visual.spec.ts', use: { viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true } },
  ],
});
