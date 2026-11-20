// Red de seguridad de cada ventana: si algo falla al pintar, en vez de quedarse en blanco
// (la barra vive todo el día), se dice y se ofrece volver a cargar (auditoría 1.0).

import { Component, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  /** Qué hacer con Esc mientras se enseña el aviso (en la barra: ocultarla). */
  onEscape?: () => void;
}

export class ErrorBoundary extends Component<Props, { failed: boolean }> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  componentDidCatch(error: unknown) {
    // Solo el error (nunca lo que se buscaba): lo ve quien abra las herramientas de desarrollo.
    console.error("Sift: fallo al pintar la ventana", error);
  }

  componentDidMount() {
    window.addEventListener("keydown", this.onKey);
  }

  componentWillUnmount() {
    window.removeEventListener("keydown", this.onKey);
  }

  /** Con el aviso a la vista, Esc hace lo de siempre en esa ventana (en la barra, ocultarla). */
  private onKey = (e: KeyboardEvent) => {
    if (this.state.failed && e.key === "Escape") this.props.onEscape?.();
  };

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <div
        role="alert"
        className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center text-fg"
      >
        <p className="text-xl font-semibold">Algo ha fallado en esta ventana</p>
        <p className="max-w-sm text-sm text-muted">
          Tus datos están bien. Vuelve a cargarla para seguir.
        </p>
        <button
          type="button"
          autoFocus
          onClick={() => window.location.reload()}
          className="rounded-item bg-selected px-4 py-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent-ink"
        >
          Volver a cargar
        </button>
      </div>
    );
  }
}
