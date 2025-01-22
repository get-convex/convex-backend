import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import { createBrowserRouter, RouterProvider } from "react-router-dom";

import "./index.css";
import App from "./App";
import Admin from "./Admin";

// @snippet start setupClient
import { ConvexProvider, ConvexReactClient } from "convex/react";

const address = import.meta.env.VITE_CONVEX_URL;

const convex = new ConvexReactClient(address);
// @snippet end setupClient

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
  },
  {
    path: "/admin",
    element: <Admin />,
  },
]);

ReactDOM.createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {/* @snippet start provideClient */}
    <ConvexProvider client={convex}>
      <RouterProvider router={router} />
    </ConvexProvider>
    {/* @snippet end provideClient */}
  </StrictMode>,
);
