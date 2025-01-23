import "./assets/main.css";

import { createApp } from "vue";
import App from "./App.vue";
import { createConvexVue } from "@convex-vue/core";

const app = createApp(App);

const convexVue = createConvexVue({
  convexUrl: import.meta.env.VITE_CONVEX_URL,
});

app.use(convexVue).mount("#app");
