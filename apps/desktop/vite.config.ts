import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// Tauri dev Aloha: fixed port 1420 must match `devUrl` in
// `src-tauri/tauri.conf.json`. `strictPort` makes a clash fail loudly
// instead of silently picking another port the shell is not serving.
export default defineConfig({
  plugins: [
    react({
      babel: {
        // Prevent browserslist from walking above the repository root in sandboxed environments
        browserslistConfigFile: false,
      },
    }),
    tailwindcss(),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2022",
  },
});
