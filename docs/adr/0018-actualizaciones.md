# ADR 0018 — Actualizaciones automáticas

- **Estado:** aceptada (2026-11-21: encendida por defecto, decidido por Darío). Preparada e inactiva hasta publicar
- **Fecha:** 2026-11-18

## Contexto

Para la 1.0 hace falta que quien instale Sift reciba las versiones nuevas sin reinstalar a mano. Pero CLAUDE.md dice que todo es local y que solo salen del PC descargas puntuales, con permiso y con el hash fijado.

## Decisión

- **`tauri-plugin-updater`**, con las versiones como Releases de GitHub y un `latest.json` firmado. El plugin comprueba la firma (minisign) con la clave pública que lleva Sift antes de instalar nada, y eso cumple el papel del «hash fijado».
- **Comprobar al arrancar, encendido por defecto; se apaga** en Ajustes → General, «Buscar actualizaciones al arrancar». Es la única conexión que Sift hace sin preguntar en el momento: solo se pide el `latest.json` (el número de la última versión), sin enviar archivos ni búsquedas.
- **Instalar siempre se pregunta**, en un diálogo de Windows (como la aprobación de comandos): «Instalar y reiniciar» o «Ahora no».
- **Inactivo hasta publicar:** el plugin solo se registra si la compilación lleva `tauri.updater.conf.json`, que solo añade el workflow de Releases. La compilación de todos los días, y la `1.0.0-rc.1` de Darío, no lo llevan.

| Opción | A favor | En contra |
|---|---|---|
| **Opcional, apagado por defecto** (propuesta) | Cumple «nada sale sin permiso» | Quien no lo active se queda en su versión |
| Encendido por defecto, con aviso en la introducción | Casi todos estarían al día | Una conexión sin pedirla expresamente |
| Sin actualizador (descargar a mano) | Cero conexiones | Nadie actualiza |

**Recomendación de Claude:** opcional y apagado, ofrecido en la introducción.

**Decisión de Darío (2026-11-21): encendido por defecto**, con dos garantías: Sift **pregunta siempre** antes de descargar e instalar, y la comprobación se puede **desactivar** en Ajustes. Lo que pesa: casi nadie activaría una opción apagada, y una versión con un arreglo de seguridad tiene que llegar. A cambio, la comprobación es la única conexión sin preguntar, y así lo cuentan el README y la sección de privacidad.

Los ajustes guardados antes conservan lo que tenían: quien lo apagó lo sigue teniendo apagado. Un fichero de ajustes sin el bloque `updates` (o una instalación nueva) lo tiene encendido (test `update_checks_are_on_unless_turned_off`). Ojo: las versiones candidatas (rc) guardaban `checkOnStartup: false` al tocar cualquier ajuste, así que quien venga de una rc lo tendrá apagado; esas compilaciones tampoco llevaban el actualizador.

## Pendiente

Generar la clave de firma y el resto de pasos del día de publicar: [docs/publicacion/pasos.md](../publicacion/pasos.md).
