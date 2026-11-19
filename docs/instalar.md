# Instalar Sift para usarlo a diario

Hay dos Sift, y no se mezclan:

| | Versión diaria (instalada) | Versión de desarrollo (`npm run tauri dev`) |
|---|---|---|
| Para qué | Usarlo de verdad | Probar cambios mientras se programan |
| Datos | `%APPDATA%\dev.sift.app` | `%APPDATA%\dev.sift.app-dev` |
| Logs | `%LOCALAPPDATA%\dev.sift.app\logs` | `%LOCALAPPDATA%\dev.sift.app-dev\logs` |
| Atajo por defecto | Alt+Espacio | Ctrl+Alt+Espacio |
| Contenido | Tus carpetas y tus Espacios | Datos de prueba: la carpeta `docs/` del repositorio y un Espacio de ejemplo |

El «dev» de `dev.sift.app` no significa desarrollo: es un identificador con forma de dominio al revés, elegido al principio. No se cambia, para no perder los datos que ya tienes.

Las pruebas, las migraciones y los fallos de la versión de desarrollo **nunca tocan tus datos reales**: cualquier compilación de depuración usa su propio identificador (ver `apps/desktop/src-tauri/src/dev.rs`). Las dos se pueden tener abiertas a la vez.

## Instalar (la primera vez)

Desde la raíz del repositorio, en `main`:

```bash
powershell -ExecutionPolicy Bypass -File scripts/instalar.ps1
```

El script:

1. Comprueba que estás en `main` sin cambios a medias y trae lo último.
2. Baja PDFium y las dependencias.
3. Compila en modo release (unos 5 minutos).
4. Abre el instalador: `target\release\bundle\nsis\Sift_<versión>_x64-setup.exe`.

Al instalarlo:

- Es un asistente en español con la imagen de Sift (ADR 0019). Al final, «Abrir Sift» lo arranca.
- **Windows puede avisar** de que el programa no está firmado (SmartScreen): «Más información» → «Ejecutar de todas formas». Firmarlo cuesta dinero y está en el backlog.
- Se instala **solo para tu usuario** (no pide permisos de administrador), en `%LOCALAPPDATA%\Sift`, con acceso en el menú Inicio.
- Si `npm run tauri dev` está abierto, ciérralo antes: el paso de las dependencias necesita reinstalarlas.

Después, en Ajustes → General puedes activar «Arrancar Sift al iniciar sesión en Windows».

## Actualizar (cuando se fusiona algo)

Lo mismo: vuelve a ejecutar el script desde `main`.

```bash
powershell -ExecutionPolicy Bypass -File scripts/instalar.ps1
```

El instalador nuevo sustituye al anterior en el mismo sitio. Si Sift está abierto, el instalador te pide cerrarlo. **Tus datos se quedan como estaban**: carpetas, Espacios, ajustes, el modelo descargado y lo que abres a menudo. Si la versión nueva trae una migración de la base de datos, se aplica sola al arrancar. Si hace falta volver a leer archivos (como cuando se añadió la página de los PDF), se hace en segundo plano.

Para solo generar el instalador sin abrirlo: `scripts/instalar.ps1 -SoloCompilar`.

## Desinstalar

Configuración de Windows → Aplicaciones → Sift → Desinstalar. Tus datos siguen en `%APPDATA%\dev.sift.app` por si vuelves a instalarlo. Si quieres borrarlo todo, borra también esa carpeta.
