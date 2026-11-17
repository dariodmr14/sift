// Iconos de la interfaz: la fuente de iconos de Windows (Segoe Fluent Icons en Windows 11,
// Segoe MDL2 Assets en Windows 10; mismos códigos). No añade dependencias, se ve igual que
// el resto del sistema y sigue el color del texto. Un solo tamaño (16 px); dentro de texto
// pequeño, `text-xs` lo baja a 12. Lucide queda solo para el icono que eliges en un Espacio.
//
// Todos comprobados en Segoe MDL2 Assets 1.79 (Windows 10): existen y tienen dibujo.

/** Código de cada glifo (los nombres son los de la documentación de Microsoft). */
export const GLYPHS = {
  settings: "", // Settings
  add: "", // Add
  close: "", // Cancel
  delete: "", // Delete
  // Fluent tiene un asa de puntos (GripperDotsVertical), pero MDL2 no: se usa el asa de
  // barras, que es la que usa Windows 10 en sus listas.
  gripper: "", // GripperBarHorizontal
  chevronRight: "", // ChevronRight
  check: "", // CheckMark
  warning: "", // Warning
  dash: "", // Remove
  enter: "", // ReturnKey
} as const;

export type GlyphName = keyof typeof GLYPHS;

export function Glyph({
  name,
  className = "",
}: {
  name: GlyphName;
  className?: string;
}) {
  return (
    <span aria-hidden className={`sift-glyph ${className}`}>
      {GLYPHS[name]}
    </span>
  );
}
