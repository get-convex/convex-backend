import { StrictMode } from "react";
import ReactDOM from "react-dom/client";
import "./index.css";
import "uplot/dist/uPlot.min.css";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
