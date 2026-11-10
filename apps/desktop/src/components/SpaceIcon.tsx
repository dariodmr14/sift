import type { Icon } from "../bindings";

/** SVG en texto → data URL, para usarlo en `<img>` o como máscara CSS. */
export function svgDataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

interface Props {
  icon: Icon;
  /** Icono real de Windows (si el icono es "automático"). */
  autoIcon?: string | null;
  /** Nombre del Espacio, para la inicial cuando no hay icono automático. */
  name: string;
  size?: number;
}

/**
 * Pinta el icono de un Espacio. Por seguridad, los SVG (de la librería o del usuario)
 * nunca se insertan como HTML: van por `<img>` o como máscara CSS, donde un `<script>`
 * dentro del SVG no se ejecuta.
 */
export function SpaceIcon({ icon, autoIcon, name, size = 24 }: Props) {
  const box = { width: size, height: size };
  switch (icon.kind) {
    case "none":
      return <span style={box} aria-hidden />;
    case "emoji":
      return (
        <span
          style={{ ...box, fontSize: size * 0.85, lineHeight: `${size}px` }}
          className="text-center"
          aria-hidden
        >
          {icon.value}
        </span>
      );
    case "library":
      // Máscara: la forma del icono, pintada con el color del texto (`currentColor`),
      // para que siga el tema igual que el resto de la UI.
      return (
        <span
          aria-hidden
          style={{
            ...box,
            backgroundColor: "currentColor",
            mask: `url("${svgDataUrl(icon.svg)}") center / contain no-repeat`,
          }}
        />
      );
    case "image":
      return (
        <img
          src={`data:${icon.mime};base64,${icon.data}`}
          alt=""
          style={box}
          draggable={false}
        />
      );
    case "auto":
      return autoIcon ? (
        <img src={autoIcon} alt="" style={box} draggable={false} />
      ) : (
        <Initial name={name} size={size} />
      );
  }
}

/** Sin icono automático (p. ej. un Espacio solo de webs): la inicial del nombre. */
function Initial({ name, size }: { name: string; size: number }) {
  return (
    <span
      aria-hidden
      style={{ width: size, height: size, fontSize: size * 0.5 }}
      className="flex items-center justify-center rounded-item bg-selected font-semibold"
    >
      {name.trim().charAt(0).toUpperCase() || "?"}
    </span>
  );
}
