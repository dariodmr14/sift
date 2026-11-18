# ADR 0018 — Actualizaciones automáticas

- **Estado:** propuesta (preparada e inactiva; el valor por defecto lo decide Darío)
- **Fecha:** 2026-11-18

## Contexto

Para la 1.0 hace falta que quien instale Sift reciba las versiones nuevas sin reinstalar a mano. Pero CLAUDE.md dice que todo es local y que solo salen del PC descargas puntuales, con permiso y con el hash fijado.

## Decisión propuesta

- **`tauri-plugin-updater`**, con las versiones como Releases de GitHub y un `latest.json` firmado. El plugin comprueba la firma (minisign) con la clave pública que lleva Sift antes de instalar nada, y eso cumple el papel del «hash fijado».
- **Opcional y apagado por defecto:** en Ajustes → General, «Buscar actualizaciones al arrancar». Es la única conexión que Sift haría sin que se lo pidas en el momento, así que la enciendes tú. Solo se envía la petición del `latest.json`: ni archivos ni búsquedas.
- **Instalar siempre se pregunta**, en un diálogo de Windows (como la aprobación de comandos): «Instalar y reiniciar» o «Ahora no».
- **Inactivo hasta publicar:** el plugin solo se registra si la compilación lleva `tauri.updater.conf.json`, que solo añade el workflow de Releases. La compilación de todos los días, y la `1.0.0-rc.1` de Darío, no lo llevan.

| Opción | A favor | En contra |
|---|---|---|
| **Opcional, apagado por defecto** (propuesta) | Cumple «nada sale sin permiso» | Quien no lo active se queda en su versión |
| Encendido por defecto, con aviso en la introducción | Casi todos estarían al día | Una conexión sin pedirla expresamente |
| Sin actualizador (descargar a mano) | Cero conexiones | Nadie actualiza |

**Recomendación:** opcional y apagado, y ofrecerlo en la introducción. Se podría añadir al paso 4 («Búsqueda inteligente»), que ya habla de lo que sale del PC.

## Pendiente de Darío

1. Aprobar el valor por defecto (apagado) y si se ofrece en la introducción.
2. Generar la clave de firma (pasos en [lanzamiento.md](../lanzamiento.md)).
