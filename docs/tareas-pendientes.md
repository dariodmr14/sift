# Tareas pendientes: preparar la publicación (sin publicar nada)

Tanda de la noche del 2026-11-21. Se vuelve a leer después de cada tarea.

**Reglas:** no cerrar el turno mientras queden tareas · decisiones a `docs/proceso/pendiente-decidir.md` (lo pequeño y reversible, «decidido sin mí») · parar solo ante algo irreversible (datos reales, Sift instalado, force push, configuración global, instalar fuera del proyecto) · nada de ventanas ni pruebas en pantalla, salvo grabar el vídeo (tarea 5, permitido expresamente) · cada bloque con CI local, PR y merge (salvo el README, que espera revisión).

## 1. Limpieza para hacerlo público (solo análisis) — hecho, PR #62; decisión 14 aplicada
- [x] Buscar en el repo y en **todo** el historial: secretos y tokens, rutas y nombres personales, el correo, capturas con datos personales y archivos internos.
- [x] Informe `docs/publicacion/limpieza.md`: qué hay, dónde y la propuesta para cada cosa (borrar, mover, dejar o reescribir el historial).
- [x] **No** aplicar nada que reescriba el historial.
- [x] CI, PR y merge.

## 2. README en dos idiomas (PR **sin fusionar**)
- [ ] `README.md` en español (principal) y `README.en.md` en inglés, con «Español | English» arriba.
- [ ] Para gente de fuera: qué es en una frase, el GIF, capturas finales de la rc.4, funciones clave, instalación (con el aviso de SmartScreen), privacidad, las cifras de la auditoría 1.0, «Cómo se ha construido» (tal como está aprobado) y créditos (unslop).
- [ ] Lo largo (aprendido, auditorías, ADR), enlazado y no copiado.
- [ ] CI y PR. **No fusionar.**

## 3. Licencia (mismo PR que el README)
- [ ] `LICENSE` «Todos los derechos reservados» en español e inglés.
- [ ] Explicada en los dos README.

## 4. Release y actualizaciones, listos pero desactivados
- [ ] Workflow que, con una etiqueta `v*`, compila el instalador, baja PDFium con su hash y crea la Release con notas.
- [ ] Actualizador configurado, a falta solo de la clave de firma.
- [ ] `docs/publicacion/pasos.md`: los pasos exactos del día de publicar (clave, dónde guardar cada parte, secreto del repo, hacerlo público, etiqueta `v1.0.0` y comprobar la Release).
- [ ] CI, PR y merge.

## 5. Vídeo en inglés
- [ ] Datos y nombres de ejemplo en inglés (nada de «Calculadora» ni carpetas en español).
- [ ] Volver a grabar las escenas reales (permitido grabar en pantalla esta noche) con la versión de desarrollo.
- [ ] Mismo montaje y misma música; renderizar `sift-lanzamiento-en.mp4`.
- [ ] Lo que toque el repo (datos de demostración en inglés, script de grabación), con CI, PR y merge. Cerrar todo lo abierto al acabar.

## Al terminar
- [ ] Resumen único: qué está fusionado, qué PR espera revisión, qué ha quedado a medias, decisiones pendientes y qué hizo cada modelo.
