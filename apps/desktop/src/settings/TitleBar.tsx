import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

/*
 * Barra de título propia de la ventana de ajustes.
 *
 * La nativa de Windows 10 es negra y no admite vidrio (el color de la barra solo se
 * puede cambiar en Windows 11). Esta se pinta sobre el mismo vidrio que el resto.
 *
 * No se pierde nada de la nativa: `data-tauri-drag-region` le pide a Windows que inicie
 * el arrastre (así funcionan el ajuste a los bordes y Win+flechas), el doble clic
 * maximiza, y los bordes siguen redimensionando. Los botones miden lo que los de
 * Windows (46 × 32 px) y usan sus mismos glifos (fuente de iconos del sistema).
 */

const GLYPH = {
  minimize: "",
  maximize: "",
  restore: "",
  close: "",
} as const;

function CaptionButton({
  label,
  glyph,
  close = false,
  onClick,
}: {
  label: string;
  glyph: string;
  close?: boolean;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      // Como en Windows, no se llega con Tab: Alt+F4, Win+↑/↓ y Alt+Espacio ya lo cubren.
      tabIndex={-1}
      onClick={onClick}
      className={`sift-caption-glyph flex h-8 w-[46px] items-center justify-center text-fg outline-none ${
        close
          ? "hover:bg-caption-close hover:text-caption-close-fg"
          : "hover:bg-hover active:bg-selected"
      }`}
    >
      <span aria-hidden>{glyph}</span>
    </button>
  );
}

export function TitleBar({ title }: { title: string }) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    const sync = () => void win.isMaximized().then(setMaximized);
    sync();
    const unlisten = win.onResized(sync);
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const win = getCurrentWindow();
  return (
    <header className="flex h-8 shrink-0 items-stretch select-none">
      <div
        data-tauri-drag-region
        className="flex min-w-0 flex-1 items-center px-3 text-xs text-muted"
      >
        {/* El texto no debe quedarse con el clic: el arrastre es de la zona entera. */}
        <span className="pointer-events-none truncate">{title}</span>
      </div>
      <CaptionButton
        label="Minimizar"
        glyph={GLYPH.minimize}
        onClick={() => void win.minimize()}
      />
      <CaptionButton
        label={maximized ? "Restaurar" : "Maximizar"}
        glyph={maximized ? GLYPH.restore : GLYPH.maximize}
        onClick={() => void win.toggleMaximize()}
      />
      <CaptionButton
        label="Cerrar"
        glyph={GLYPH.close}
        close
        onClick={() => void win.close()}
      />
    </header>
  );
}
