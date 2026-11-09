# ADR 0004 — Espacios en SQLite: esquema, migraciones y aprobaciones de comandos

- **Estado:** aceptada
- **Fecha:** 2026-11-09

## Contexto

Los Espacios se adelantan a la Fase 1.5 (ver [FEEDBACK.md](../../FEEDBACK.md)). El ADR 0001 fija SQLite como fuente de verdad, así que hay que decidir cómo se guardan los Espacios, cómo evoluciona el esquema y dónde se recuerda qué comandos ya se han confirmado.

## Decisiones

**1. Espacios en SQLite (`sift-storage`), como dice el plan.** Tablas `spaces`, `space_aliases` y `space_actions`. Cada escritura va en una transacción: si algo falla a mitad, no queda un Espacio con la mitad de sus acciones. Los alias y las acciones se borran y reinsertan al actualizar. Son listas cortas, y así el orden guardado es siempre el de la UI.

**2. Cada acción se guarda como JSON en su fila, con una columna `kind` aparte.** Los tipos de acción tienen campos distintos (una URL frente a programa, argumentos y carpeta). Hay dos alternativas, una tabla por tipo o una tabla ancha llena de columnas nulas, y las dos complican añadir tipos nuevos, como las acciones de ventana de la Fase 6.5. El JSON lo valida el mismo tipo de Rust (`serde`) al leer y al escribir. `kind` existe por si hace falta consultar por tipo.

**3. Migraciones con `PRAGMA user_version`, sin librería.** Es un entero que SQLite guarda en el fichero. Al abrir, se aplican en orden las migraciones que falten, cada una en su transacción. Si la base de datos es de una versión más nueva de Sift, no se toca y se avisa. Son 40 líneas que se entienden enteras; una librería de migraciones no aportaría nada con este volumen.

**4. Aprobación de comandos ligada al comando exacto.** La tabla `command_approvals` guarda (programa, argumentos, carpeta). Si cambias cualquiera de los tres, es otro comando y se vuelve a pedir confirmación. Aprobar "el Espacio" no sería suficiente: alguien podría cambiar un argumento después de aprobarlo. Las aprobaciones **no se exportan**: un Espacio importado de otro PC pide confirmación la primera vez.

**5. Las imágenes de icono van dentro del Espacio, en base64.** Así un Espacio exportado lleva su icono y no hay ficheros sueltos que se pierdan. Se limitan a ~256 KB. Los iconos de la librería guardan también su SVG, para que la barra no cargue la librería entera.

**6. Los ajustes simples (atajo, "solo texto") siguen en `settings.json`.** El atajo se necesita al arrancar, antes que nada. El fichero es legible y editable a mano, y no tiene relaciones ni historial. Si los ajustes crecen o necesitan transacciones, se moverán a SQLite con una migración.

## Consecuencias

- Una migración publicada no se edita nunca: los cambios van en una nueva al final de `MIGRATIONS`.
- Los tests del repositorio usan SQLite en memoria (`Database::open_in_memory`) y son rápidos.
- `rusqlite` con `bundled` compila SQLite dentro de la app: no depende de la versión de Windows.
