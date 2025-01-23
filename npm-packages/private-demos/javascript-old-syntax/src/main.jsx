import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import App from "./App";
// @snippet start setupClient
import { ConvexProvider, ConvexReactClient } from "convex/react";

const address = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(address);
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
