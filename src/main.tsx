import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { EdgeDock } from "./features/edgeDock/EdgeDock";
import "./styles/global.css";

const surface = new URLSearchParams(window.location.search).get("surface");
if (surface === "desktop-sticky") document.documentElement.dataset.surface = "desktop-sticky";
if (surface === "edge-dock") document.documentElement.dataset.surface = "edge-dock";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {surface === "desktop-sticky" ? (
      <App surface="desktop" />
    ) : surface === "edge-dock" ? (
      <EdgeDock />
    ) : (
      <App />
    )}
  </StrictMode>,
);
