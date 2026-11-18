// Números y tiempos como los diría una persona. Todo lo que la app enseña pasa por aquí:
// nada de «335 s» ni «1240 archivos».

/** «documentos de Word» → «Documentos de Word», para cuando el texto va solo. */
export function capitalize(text: string): string {
  return text.charAt(0).toLocaleUpperCase("es") + text.slice(1);
}

/** 1240 → «1.240». (`toLocaleString("es")` no pone el punto con 4 cifras: aquí sí.) */
export function count(n: number): string {
  return String(Math.round(n)).replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}

/** «1 archivo», «1.240 archivos». */
export function files(n: number): string {
  return `${count(n)} ${n === 1 ? "archivo" : "archivos"}`;
}

/**
 * Cuánto dura algo, con las dos unidades más grandes: «menos de un segundo», «35 s»,
 * «5 min 35 s», «2 min», «1 h 5 min». Nunca «335 s».
 */
export function duration(seconds: number): string {
  if (seconds < 1) return "menos de un segundo";
  const total = Math.round(seconds);
  const h = Math.floor(total / 3600);
  const min = Math.floor((total % 3600) / 60);
  const s = total % 60;
  const parts =
    h > 0
      ? [`${h} h`, min > 0 ? `${min} min` : ""]
      : min > 0
        ? [`${min} min`, s > 0 ? `${s} s` : ""]
        : [`${s} s`];
  return parts.filter(Boolean).join(" ");
}

/**
 * Hace cuánto pasó algo, a partir de una fecha ISO: «hace un momento», «hace 15 min»,
 * «hace 3 h», «ayer», «hace 4 días». Redondea hacia abajo: 59 min no es «hace 1 h».
 */
export function since(
  iso: string | null | undefined,
  now = Date.now(),
): string {
  if (!iso) return "aún no";
  const secs = Math.max(0, Math.round((now - Date.parse(iso)) / 1000));
  if (secs < 60) return "hace un momento";
  if (secs < 3600) return `hace ${Math.floor(secs / 60)} min`;
  if (secs < 86400) return `hace ${Math.floor(secs / 3600)} h`;
  const days = Math.floor(secs / 86400);
  return days === 1 ? "ayer" : `hace ${count(days)} días`;
}
