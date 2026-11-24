# Tareas pendientes: preparar la publicación (sin publicar nada)

Tanda de la noche del 2026-11-21. Se vuelve a leer después de cada tarea.

**Terminada el 2026-11-24.** Dónde ha quedado todo: [estado-sesion.md](estado-sesion.md).

**Reglas** (actualizadas el 2026-11-21 por la mañana: Darío está despierto y deja grabar en pantalla sin preguntar): no cerrar el turno mientras queden tareas · decisiones a `docs/proceso/pendiente-decidir.md` (lo pequeño y reversible, «decidido sin mí») · parar solo ante algo irreversible (datos reales, Sift instalado, force push, configuración global, instalar fuera del proyecto) · nada de ventanas ni pruebas en pantalla, salvo grabar el vídeo (tarea 5, permitido expresamente) · cada bloque con CI local, PR y merge (salvo el README, que espera revisión).

## 1. Limpieza para hacerlo público (solo análisis) — hecho, PR #62; decisión 14 aplicada
- [x] Buscar en el repo y en **todo** el historial: secretos y tokens, rutas y nombres personales, el correo, capturas con datos personales y archivos internos.
- [x] Informe `docs/publicacion/limpieza.md`: qué hay, dónde y la propuesta para cada cosa (borrar, mover, dejar o reescribir el historial).
- [x] **No** aplicar nada que reescriba el historial.
- [x] CI, PR y merge.

## 2. README en dos idiomas (PR **sin fusionar**) — PR #65 con los GIF de la rc.4 y la CI en verde; espera la revisión de Darío
- [x] `README.md` en español (principal) y `README.en.md` en inglés, con «Español | English» arriba.
- [x] Para gente de fuera (los GIF, de la rc.4 y con los datos de cada idioma): qué es en una frase, el GIF, capturas finales de la rc.4, funciones clave, instalación (con el aviso de SmartScreen), privacidad, las cifras de la auditoría 1.0, «Cómo se ha construido» (tal como está aprobado) y créditos (unslop).
- [x] Lo largo (aprendido, auditorías, ADR), enlazado y no copiado.
- [x] CI y PR (#65). **No fusionar.**

## 3. Licencia (mismo PR que el README) — hecho en la rama, con «sin garantías» y Darío Morales Riega (decisión 16)
- [x] `LICENSE` «Todos los derechos reservados» en español e inglés.
- [x] Explicada en los dos README.

## 4. Release y actualizaciones, listos pero desactivados — hecho, PR #63; decisión 15 aplicada (comprobación encendida por defecto)
- [x] Workflow que, con una etiqueta `v*`, compila el instalador, baja PDFium con su hash y crea la Release con notas.
- [x] Actualizador configurado, a falta solo de la clave de firma.
- [x] `docs/publicacion/pasos.md`: los pasos exactos del día de publicar (clave, dónde guardar cada parte, secreto del repo, hacerlo público, etiqueta `v1.0.0` y comprobar la Release).
- [x] CI, PR y merge.

## 5. Vídeo en inglés — hecho; y el de español, rehecho con la rc.4 (Video Shotcraft, real-sound y Earshot)
- [x] Datos y nombres de ejemplo en inglés (nada de «Calculadora» ni carpetas en español).
- [x] Volver a grabar las escenas reales (permitido grabar en pantalla esta noche) con la versión de desarrollo.
- [x] Mismo montaje y misma música; `sift-lanzamiento-en.mp4` renderizado (y una versión sin música).
- [x] Lo que toque el repo (datos de demostración en inglés, scripts de capturas), con CI, PR y merge (#64). Todo cerrado al parar.
- [x] Arreglo de `capturas-intro.mjs` y capturas de la introducción de la rc.4 (#66).
- [x] Pasada de planos y sonido en los dos idiomas, revisión independiente y sus arreglos; GIF de los dos README.

## Al terminar
- [x] Resumen único (2026-11-24): qué está fusionado, qué PR espera revisión, qué ha quedado a medias, decisiones pendientes, qué hizo cada modelo y qué le falta a Darío para publicar.
