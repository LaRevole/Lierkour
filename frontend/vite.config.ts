import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    // Bind explicitly to IPv4 so the URL matches Tauri's `devUrl` on both
    // Windows and Linux. Without this, `localhost` may resolve to `::1`
    // while Vite listens on `127.0.0.1` (or vice versa), which shows up as
    // "Could not connect to localhost: Connection refused".
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
});
