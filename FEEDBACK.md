# Feedback de uso

Lo que va saliendo al usar Sift. Cada punto acaba en una tarea de [PLAN.md](PLAN.md) o en el backlog.

Formato: fecha, qué pasa o qué falta, y a dónde va.

## 2026-11-09 — Tras probar la Fase 1

- **Ajustes desde la barra.** Hoy solo se llega por el icono de la bandeja. Debería poder entrarse desde la propia barra: `Ctrl+,`, un icono discreto o escribiendo "ajustes". → Fase 1.5.
- **Espacios reales antes que la búsqueda.** Los resultados de ejemplo no sirven para el día a día; los Espacios básicos no dependen del indexado y se pueden adelantar. → Fase 1.5 (adelantada de la Fase 6).
  - Sin datos de ejemplo: si no hay Espacios, estado vacío con un atajo para crear el primero.
  - Editor en Ajustes: crear, editar, duplicar, borrar y reordenar.
  - Cada Espacio: nombre, alias, icono y lista ordenada de acciones (carpeta, archivo, URL, aplicación con selector, comando con programa + argumentos + carpeta de trabajo). Botón "probar" por acción.
  - Seguridad: sin shell, y confirmación la primera vez que se ejecuta un comando.
  - Persistencia en SQLite con migraciones. Exportar/importar a JSON.
- **Iconos personalizables.** Por Espacio: librería con buscador, emoji, imagen propia (PNG/SVG/ICO) o ninguno. Opción global "solo texto". Icono real de Windows por defecto para apps y archivos. → Fase 1.5.
- **Más personalidad visual.** Simple pero más bonita, que luzca en un vídeo para LinkedIn. Primero 2–3 direcciones visuales para elegir; después, sistema de diseño con tokens. Animaciones rápidas que no afecten a los < 100 ms y que respeten "reducir movimiento". → Fase 1.5 (adelanta parte de la Fase 7).
- **Colocar ventanas.** Acciones que abren una app en una zona de la pantalla (p. ej. "VS Code en la mitad derecha"), como FancyZones: esperar a que abra, identificar su ventana, varios monitores. → Fase 6.5 (nueva).
