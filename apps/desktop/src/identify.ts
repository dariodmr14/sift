// «Identificar» en el editor de acciones: una ventanita en cada pantalla con su número de
// Windows, unos segundos (la cierra Rust). Sin React: tiene que salir al instante.

import "./index.css";

declare global {
  interface Window {
    /** Lo pone Rust al crear la ventana (`initialization_script`). */
    __siftScreen?: number;
  }
}

const n = window.__siftScreen;
const el = document.getElementById("screen");
if (el && typeof n === "number") {
  el.textContent = String(n);
  document.title = `Pantalla ${n}`;
}
