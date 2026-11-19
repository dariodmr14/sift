// El indicador de carga de Sift: las tres barras del tamiz (el icono) abriéndose y
// cerrándose por turnos. Para lo que tarda de verdad: el primer indexado, la descarga de
// la búsqueda inteligente o una vista previa lenta. Con «reducir movimiento», un pulso
// suave de opacidad. Documentado en docs/design-system.md.

import { useEffect, useState } from "react";

/** Las barras, sin texto: decorativas; el texto de al lado dice qué pasa. */
export function SiftLoader({ size = "sm" }: { size?: "sm" | "md" }) {
  return (
    <span aria-hidden className={`sift-loader sift-loader-${size}`}>
      <span />
      <span />
      <span />
    </span>
  );
}

/**
 * Las barras con un texto que se anuncia («Preparando la vista previa…»). Con `delay`,
 * no sale hasta pasado ese tiempo: lo rápido no parpadea.
 */
export function SiftLoading({
  label,
  delay = 0,
  size = "sm",
}: {
  label: string;
  delay?: number;
  size?: "sm" | "md";
}) {
  const [shown, setShown] = useState(delay === 0);
  useEffect(() => {
    if (delay === 0) return;
    const t = setTimeout(() => setShown(true), delay);
    return () => clearTimeout(t);
  }, [delay]);
  if (!shown) return null;
  return (
    <p
      role="status"
      className={`flex items-center gap-2 text-sm text-muted ${
        size === "md" ? "flex-col justify-center py-6" : ""
      }`}
    >
      <SiftLoader size={size} />
      {label}
    </p>
  );
}
