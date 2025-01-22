import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ConvexProvider, ConvexReactClient } from "convex/react";
import { SessionProvider } from "./hooks/useServerSession";

const convex = new ConvexReactClient(import.meta.env.VITE_CONVEX_URL);

const root = ReactDOM.createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <ConvexProvider client={convex}>
      <SessionProvider storageLocation={"sessionStorage"}>
        <App />
      </SessionProvider>
    </ConvexProvider>
  </StrictMode>,
);
