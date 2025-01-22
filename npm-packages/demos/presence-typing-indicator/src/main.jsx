import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";
// @snippet start setupClient
import { ConvexProvider, ConvexReactClient } from "convex/react";

const convex = new ConvexReactClient(import.meta.env.VITE_CONVEX_URL);
// @snippet end setupClient

ReactDOM.createRoot(document.getElementById("root")).render(
  <StrictMode>
    {/* @snippet start provideClient */}
    <ConvexProvider client={convex}>
      <App />
    </ConvexProvider>
    {/* @snippet end provideClient */}
  </StrictMode>,
);
