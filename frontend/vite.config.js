import { fileURLToPath } from "node:url";
import path from "node:path";
import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// The Rust/Tauri project dir (../src relative to this file) — an absolute
// path, not a bare "**/src/**" glob, because this repo has two directories
// named `src`: this one (frontend/src, which must be watched for HMR) and
// the sibling Rust crate (../src, which must not be).
const rustProjectDir = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src");

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching the Tauri/Rust project dir
      ignored: [`${rustProjectDir}/**`],
    },
  },
}));
