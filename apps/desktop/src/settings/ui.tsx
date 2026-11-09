// Piezas de UI compartidas de la ventana de ajustes. Cuando se elija la dirección visual
// (punto 5), se cambian aquí y todo el editor la sigue.

import type {
  ButtonHTMLAttributes,
  InputHTMLAttributes,
  ReactNode,
} from "react";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-(--accent) text-black font-medium hover:brightness-110",
  secondary: "bg-(--surface-hover) hover:bg-(--surface-selected)",
  ghost: "hover:bg-(--surface-hover)",
  danger: "text-red-400 hover:bg-red-500/15",
};

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant }) {
  return (
    <button
      type="button"
      className={`rounded-md px-3 py-1.5 text-sm whitespace-nowrap outline-none focus-visible:ring-2 focus-visible:ring-(--accent) disabled:opacity-40 ${VARIANTS[variant]} ${className}`}
      {...props}
    />
  );
}

export function TextInput({
  className = "",
  ...props
}: InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      spellCheck={false}
      className={`w-full rounded-md border border-(--border) bg-(--surface-hover) px-3 py-1.5 text-sm outline-none focus:border-(--accent) ${className}`}
      {...props}
    />
  );
}

export function Field({
  label,
  hint,
  children,
}: {
  label: string;
  hint?: ReactNode;
  children: ReactNode;
}) {
  return (
    <label className="block space-y-1">
      <span className="text-xs font-medium text-(--text-muted)">{label}</span>
      {children}
      {hint && (
        <span className="block text-xs text-(--text-muted)">{hint}</span>
      )}
    </label>
  );
}

/** Ventana modal sencilla (selector de apps, confirmaciones). Esc la cierra. */
export function Modal({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div
      className="fixed inset-0 z-10 flex items-center justify-center bg-black/50 p-6"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
      onKeyDown={(e) => e.key === "Escape" && onClose()}
    >
      <div
        role="dialog"
        aria-modal
        aria-label={title}
        className="flex max-h-full w-full max-w-lg flex-col rounded-lg border border-(--border) bg-(--surface) shadow-2xl"
      >
        <header className="flex items-center justify-between border-b border-(--border) px-4 py-3">
          <h3 className="font-medium">{title}</h3>
          <Button variant="ghost" onClick={onClose} aria-label="Cerrar">
            ✕
          </Button>
        </header>
        {children}
      </div>
    </div>
  );
}

export function ErrorText({ children }: { children: ReactNode }) {
  return (
    <p role="alert" className="text-sm text-red-400">
      {children}
    </p>
  );
}
