import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import dns from "dns";

// For Node <17
dns.setDefaultResultOrder("verbatim");

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
});
