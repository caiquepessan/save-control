import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// A interface fica em ui/; o build vai para dist/ (lido pelo Tauri).
export default defineConfig({
  root: "ui",
  plugins: [react()],
  build: { outDir: "../dist", emptyOutDir: true, target: "es2022" },
  server: { port: 5173, strictPort: true },
  clearScreen: false,
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    globals: true,
    css: false,
  },
});
