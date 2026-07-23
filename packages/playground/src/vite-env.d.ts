/// <reference types="vite/client" />

/** Build-time provenance injected by Vite `define` (see vite.config.ts). */
declare const __BUILD_INFO__: {
  jsVersion: string;
  jsBundle: string;
  jsBundleGz: string;
  rsVersion: string;
  compatVersion: string;
  compatBundle: string;
  compatBundleGz: string;
};
