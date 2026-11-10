import React from "react";
import ReactDOM from "react-dom/client";
import Settings from "./Settings";
import "../index.css";
import { applyAppearance, applyWindowMaterial } from "../appearance";

// Mismo tema, acento y vidrio que la barra.
void applyAppearance();
void applyWindowMaterial();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Settings />
  </React.StrictMode>,
);
