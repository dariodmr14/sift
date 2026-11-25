// Ajustes → Estado: si todo va bien, de un vistazo. Una tarjeta por cosa que le importa a
// quien usa Sift, con su tono (bien, en marcha, aviso, fallo, apagado). Lo técnico va en
// «Avanzado». Lógica pura, para probarla.

import type { IndexStatus, Status } from "../bindings";
import { count, files } from "../format";
import { shortcutLabel } from "../shortcut";
import { statusLine } from "./folders";

export type Tone = "ok" | "working" | "warning" | "error" | "off";

export interface StatusCard {
  id: "hotkey" | "folders" | "smart" | "data";
  tone: Tone;
  title: string;
  detail: string;
}

/** «Alt+Space» → «Alt+Espacio», como se ve en el teclado español. */
export const hotkeyLabel = shortcutLabel;

export function statusCards(
  status: Status,
  index: IndexStatus | null,
  roots: { folders: number; files: number },
): StatusCard[] {
  const cards: StatusCard[] = [];

  cards.push(
    status.hotkey
      ? {
          id: "hotkey",
          tone: "ok",
          title: "Atajo funcionando",
          detail: `${hotkeyLabel(status.hotkey)} abre la barra desde cualquier sitio.`,
        }
      : {
          id: "hotkey",
          tone: "error",
          title: "El atajo no funciona",
          detail:
            "Windows no dejó usarlo, quizá porque otra app lo tiene. Prueba con otro en General.",
        },
  );

  const line = statusLine(index, roots.files, roots.folders);
  cards.push(
    roots.folders === 0
      ? {
          id: "folders",
          tone: "warning",
          title: "Sin carpetas",
          detail: "Añade una carpeta en Carpetas para buscar en tus archivos.",
        }
      : index?.error
        ? {
            id: "folders",
            tone: "error",
            title: "Las carpetas no se pudieron leer",
            detail: index.error,
          }
        : index && !index.running
          ? {
              id: "folders",
              tone: "ok",
              title: "Carpetas al día",
              detail: `${files(roots.files)} listos para buscar.`,
            }
          : {
              id: "folders",
              tone: "working",
              title: "Leyendo tus carpetas",
              detail: line.text,
            },
  );

  const smart = index?.smart;
  cards.push(
    !smart
      ? {
          id: "smart",
          tone: "working",
          title: "Búsqueda inteligente",
          detail: "Comprobando…",
        }
      : !smart.enabled
        ? {
            id: "smart",
            tone: "off",
            title: "Búsqueda inteligente desactivada",
            detail:
              "Sift busca por nombre y por las palabras que contienen tus archivos.",
          }
        : smart.awaitingPermission
          ? {
              id: "smart",
              tone: "warning",
              title: "Búsqueda inteligente sin descargar",
              detail: "Necesita tu permiso para descargarse. Está en General.",
            }
          : smart.note
            ? {
                id: "smart",
                tone: "warning",
                title: "Búsqueda inteligente con problemas",
                detail: smart.note,
              }
            : smart.ready
              ? {
                  id: "smart",
                  tone: "ok",
                  title: "Búsqueda inteligente lista",
                  detail:
                    "Encuentra tus archivos también por lo que significan.",
                }
              : {
                  id: "smart",
                  tone: "working",
                  title: "Preparando la búsqueda inteligente",
                  detail:
                    smart.total > 0
                      ? `${count(smart.done)} de ${count(smart.total)}, cuando el PC está libre. Ya puedes buscar.`
                      : "Cuando el PC está libre.",
                },
  );

  cards.push(
    status.storageError
      ? {
          id: "data",
          tone: "error",
          title: "Tus cambios no se guardan",
          detail: status.storageError,
        }
      : {
          id: "data",
          tone: "ok",
          title: "Tus datos se guardan bien",
          detail: "Todo se queda en este PC.",
        },
  );
  return cards;
}
