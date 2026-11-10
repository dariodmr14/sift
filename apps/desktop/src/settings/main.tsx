import React from "react";
import ReactDOM from "react-dom/client";
import Settings from "./Settings";
import "../index.css";
import { applyAppearance } from "../appearance";

// Mismo tema y acento que Windows (la ventana de ajustes es opaca: sin Acrylic).
void applyAppearance();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Settings />
  </React.StrictMode>,
);
