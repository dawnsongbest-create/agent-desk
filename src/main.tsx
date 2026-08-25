import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { DesktopSticky } from "./features/desktopSticky/DesktopSticky";
import "./styles/global.css";

const surface = new URLSearchParams(window.location.search).get("surface");
if (surface === "desktop-sticky") document.documentElement.dataset.surface = "desktop-sticky";

createRoot(document.getElementById("root")!).render(
  <StrictMode>{surface === "desktop-sticky" ? <DesktopSticky /> : <App />}</StrictMode>,
);
