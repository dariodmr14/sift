import React from "react";
import ReactDOM from "react-dom/client";
import Settings from "./Settings";
import "../index.css";
import { applyAppearance, applyWindowMaterial } from "../appearance";
import { commands, events } from "../bindings";
import { ErrorBoundary } from "../components/ErrorBoundary";

// Mismo tema, acento y vidrio que la barra. La ventana se crea oculta: se le dice a Rust
// que la muestre cuando ya hay tema, vidrio y un primer render, para que el primer
// fotograma visible sea el bueno. (`requestAnimationFrame` puede no dispararse con la
// ventana oculta: por eso también hay un temporizador.)
void Promise.allSettled([applyAppearance(), applyWindowMaterial()]).then(() => {
  let told = false;
  const ready = () => {
    if (told) return;
    told = true;
    void commands.settingsReady();
  };
  requestAnimationFrame(() => requestAnimationFrame(ready));
  setTimeout(ready, 60);
});

// Mientras la ventana se mueve, un velo tapa el cambio de material (ver index.css).
void events.glassMoving.listen((e) => {
  document.documentElement.dataset.moving = String(e.payload);
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <Settings />
    </ErrorBoundary>
  </React.StrictMode>,
);
