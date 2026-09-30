import { fileURLToPath, URL } from "node:url";
import process from "node:process";
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  test: {
    environment: "jsdom",
    include: ["src/**/*.test.{ts,tsx}"],
    restoreMocks: true,
  },

  // Opções específicas para `tauri dev` / `tauri build`:
  // 1. não esconder os erros do Rust
  clearScreen: false,
  // 2. o Tauri espera uma porta fixa; falhar se estiver ocupada
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
      // 3. não observar o `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
