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

## 2026-11-10 — Tras usar los Espacios (rama `feat/pulido-ajustes`)

- ✅ **Vidrio en todas las ventanas.** Ajustes y sus diálogos también son de vidrio, con la misma alternativa opaca. Como en Fluent, cada superficie lleva su material: Acrylic en la barra; Mica (Windows 11) o blur con tinte (Windows 10) en Ajustes, porque Acrylic va a tirones al arrastrar o redimensionar en Windows 10; y vidrio por CSS en diálogos y barras fijas. Resuelto.
- ✅ **«Nuevo Espacio» al final de la lista**, donde aparecerá el Espacio creado. Resuelto.
- ✅ **Icono como avatar.** El icono va a la izquierda, con nombre y alias a su derecha (tarjeta de contacto); clic en el avatar para cambiarlo. Resuelto.
- ✅ **Reordenar arrastrando** Espacios y acciones, con línea del acento donde caerá; Alt+↑/↓ con teclado. El orden de los Espacios se guarda en SQLite al soltar; el de las acciones, al guardar el Espacio. Resuelto.

## 2026-11-10 — Feedback tras usar feat/pulido-ajustes

### Bugs
- [x] ✅ **Ctrl+, no abre Ajustes** (resuelto en `feat/fase-2-indexado`: se detecta por tecla física `e.code`; Ctrl+N abre Ajustes en «Nuevo Espacio», que es lo esperado) desde la barra (teclado español). Ctrl+N sí abre Ajustes. Flechas, Enter y Esc funcionan bien. Revisar cómo se detecta la tecla coma con distribuciones no inglesas (usar `event.code` y no solo `event.key`, o lo que corresponda) y comprobar que Ctrl+N hace lo que debe.
- [x] ✅ **El vidrio de Ajustes y el de la barra no se parecen** (tintes ajustados por material): Ajustes es mucho más translúcido. Deben verse coherentes.

### Apariencia: vidrio vs opaco
- [x] ✅ **Barra principal**: vidrio, pero menos translúcido que ahora, para que el fondo no choque ni reste legibilidad.
- [x] ✅ **Ajustes y diálogos: opacos por defecto.** Para algunas personas el vidrio en ventanas de trabajo es incómodo.
- [x] ✅ **Opción en Ajustes → Apariencia**: "Vidrio en toda la app" (desactivada por defecto). Si se activa, Ajustes y diálogos usan vidrio con la misma intensidad que la barra.
- [x] ✅ Dejarlo preparado para un futuro sistema de temas (bloque `appearance` en los ajustes) y personalización de la apariencia (no hacerlo ahora, solo no cerrarle la puerta).

### Editor de Espacios
- [x] ✅ **"Nuevo Espacio"**: sacarlo de la lista y ponerlo como botón fijo justo encima de "Importar" / "Exportar", con su mismo formato. Con muchos Espacios, al final de la lista quedaría inaccesible.
- [x] ✅ **Quitar el arrastre para reordenar Espacios.** (se mantiene Alt+↑/↓ hasta el orden por uso de la Fase 5) Solo cambia el orden de la lista y no aporta nada. Valorar si mantener Alt+↑/↓ o quitar también el orden manual (por ejemplo, orden alfabético o por uso).
- [ ] ⏳ **Arrastre de acciones con animación real** — pendiente para la Fase 7 (pulido): coste medio y necesita afinado visual con ratón real.: al arrastrar, la tarjeta se mueve con el puntero y las demás se desplazan con una animación suave para hacerle hueco (tipo listas de Notion o Linear), no solo una línea que marca dónde cae. Respetar "reducir movimiento" de Windows.
- [x] ✅ **Avatar y campos desalineados** (avatar de 88 px = bloque de campos 40+12+36, en la rejilla de 4 px): el icono (80 px) es más bajo que el bloque de Nombre + Alias, así que queda a una altura rara. Ajustarlo aplicando buenas prácticas de diseño de interfaces (alineación, ritmo vertical y espaciado del sistema Vidrio): por ejemplo, que el avatar ocupe la altura del bloque o que ambos queden alineados arriba con una relación de tamaños coherente. Decidir y justificar.
## 2026-11-11 — Tras usar la Fase 2 (búsqueda)

### Vidrio
- [ ] **Vidrio único.** El de la barra (Alt+Espacio) es el bueno: translúcido, se intuye lo de detrás sin ser transparente. Ajustes y diálogos tienen que verse *exactamente* igual, no parecido. Comparar con capturas una al lado de la otra.
- [ ] **Por defecto, toda la app con vidrio.** La opción pasa a ser «Desactivar vidrio» (todo opaco).

### Editor de Espacios
- [ ] **Arrastre de acciones con animación real, ya** (no en la Fase 7): la tarjeta sigue al puntero y las demás se apartan suavemente para hacerle hueco (estilo Notion/Linear). Mejor con una librería probada (p. ej. dnd-kit). Darío lo prueba con ratón real.
- [ ] **Al arrastrar por el asa ⠿ se selecciona texto o el icono.** Que el asa y la tarjeta no seleccionen mientras se arrastra, sin romper la edición de los campos.

### Ajustes → Carpetas, para cualquiera
- [ ] Los patrones a ignorar vienen por defecto y van en «Avanzado», plegado, con una explicación en lenguaje normal.
- [ ] Estado en lenguaje humano: «Al día · 1.240 archivos», «Buscando cambios…», «Leyendo 32 de 120 archivos», con barra de progreso.
- [ ] Por carpeta: nombre, ruta abreviada, número de archivos y última actualización.
