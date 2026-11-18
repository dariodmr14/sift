# Propuesta: introducción guiada (primera vez)

> Propuesta para que Darío la apruebe antes de implementarla (FEEDBACK 2026-11-18, punto 6). No hay código todavía.

Sustituye a la lista de pasos de la bienvenida actual: **un paso por pantalla**, en la misma ventana de la primera vez (la de Ajustes, sin navegación). Arriba, el progreso; abajo, **Atrás · Siguiente** a la derecha y **Saltar la introducción** a la izquierda. Enter = Siguiente; Esc no hace nada (para no saltarla sin querer).

## Los pasos

| # | Pantalla | Qué hace la persona | Cómo se comprueba |
|---|---|---|---|
| 1 | **Hola** · «Sift encuentra tus archivos y abre de golpe lo que usas.» | Leer y pulsar Siguiente | — |
| 2 | **Tu atajo** · «Pulsa Alt+Espacio ahora.» | Pulsar el atajo: se abre la barra de verdad; Esc la cierra y vuelve aquí | Rust ya avisa cuando la barra se abre (`launcherShown`): la tecla dibujada se «hunde», sale ✓ y se pasa solo al paso 3. Con «Cambiar…» al lado y «No puedo pulsarlo» para seguir |
| 3 | **Dónde buscar** · Documentos, Escritorio u otra carpeta | Elegir una o varias | Empieza a leerlas en segundo plano; una barra de progreso real (la de Carpetas) |
| 4 | **Prueba a buscar** · una caja como la de la barra dentro del paso | Escribir algo | Resultados reales de lo ya leído (la búsqueda de siempre). Si aún no hay nada leído: «En cuanto termine de leer, saldrá aquí» y se puede seguir |
| 5 | **Búsqueda inteligente** · qué es, en una frase, y que se descarga una vez (~120 MB, con su hash) | Descargar o «Ahora no» | Como hoy: nada sale del PC sin este permiso |
| 6 | **Tu primer Espacio** · un mini editor: nombre y 1–2 acciones sugeridas (la carpeta del paso 3, una web) | Crearlo o saltarlo | Se guarda como cualquier Espacio; se puede editar luego en Ajustes |
| 7 | **Listo** · «Pulsa Alt+Espacio para empezar» | Cerrar | Igual que el final de hoy |

**Repetir:** Ajustes → Primeros pasos → «Ver la introducción otra vez». Lo ya hecho sale marcado (carpetas elegidas, modelo descargado) y no se repite nada.

## Qué se anima y cómo

Todo con los tokens de duración de Vidrio, solo opacidad y transformaciones, y **nada** con «reducir movimiento» (el paso cambia sin transición). No afecta a la apertura de la barra: va en la ventana de Ajustes.

- **Cambio de paso:** el que sale se desvanece y se desplaza 16 px; el que entra llega desde 16 px al otro lado (derecha con Siguiente, izquierda con Atrás). ~200 ms, *ease-out*. El foco pasa al título del paso nuevo y el lector de pantalla oye «Paso 3 de 7: Dónde buscar».
- **Progreso:** 7 segmentos finos arriba; el actual se llena con el acento (`scaleX` de 0 a 1, 200 ms). Los hechos quedan llenos.
- **Éxito (atajo pulsado, carpeta añadida, Espacio creado):** el ✓ aparece con escala 0,9 → 1 y opacidad, 150 ms. Sin rebotes ni confeti.
- **Paso 2:** la tecla dibujada (`Keycap`) baja 1 px y se oscurece al detectar la pulsación, como una tecla de verdad.
- **Paso 4:** los resultados entran con la misma animación de lista que la barra.

## Decisiones que necesito de ti
1. **¿Los 7 pasos, o quitamos alguno?** Mi recomendación: los 7, con 4 y 6 opcionales (Siguiente sirve sin hacer nada).
2. **Paso 4: ¿caja dentro de la introducción, o pedir que abras la barra de verdad?** Recomiendo dentro: no depende de que el foco vaya y vuelva entre ventanas, y se ve el resultado al lado de la explicación.
3. **¿«Saltar la introducción» pide confirmación?** Recomiendo que no: se puede repetir desde Ajustes.
