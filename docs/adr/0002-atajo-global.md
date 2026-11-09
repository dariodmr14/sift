# ADR 0002 — Atajo global: RegisterHotKey con hook de teclado como alternativa

- **Estado:** aceptada
- **Fecha:** 2026-11-08

## Contexto

Sift se abre con `Alt+Espacio` desde cualquier app, y tiene que aparecer en < 100 ms. Windows usa `Alt+Espacio` para el menú de ventana de la app que tienes delante, y otra app (PowerToys Run, por ejemplo) puede tener ya ese atajo. El atajo tiene que ser configurable.

## Decisión

Un hilo propio con bucle de mensajes de Win32 (`src-tauri/src/hotkey.rs`), con dos vías:

1. **`RegisterHotKey`** (la oficial). Windows detecta la combinación, nos manda `WM_HOTKEY` y se "come" la pulsación, así que no se abre el menú de ventana. `MOD_NOREPEAT` evita repeticiones al mantenerla pulsada.
2. Si falla (combinación ocupada o reservada), **hook de teclado de bajo nivel** (`WH_KEYBOARD_LL`), como hace PowerToys Run:
   - Lleva la cuenta de los modificadores que ve pasar y se traga la combinación (y su keyup).
   - Ignora el autorepeat.
   - Inyecta una tecla falsa (VK `0xE8`) para que al soltar Alt/Win no se active la barra de menús o el menú Inicio.
   - Se puede forzar con `SIFT_FORCE_KEYBOARD_HOOK=1` para probarlo.

No se usa `tauri-plugin-global-shortcut`. Por dentro usa `RegisterHotKey`, pero no ofrece la alternativa del hook, y tener las dos vías en el mismo hilo y con los mismos códigos de tecla simplifica el diseño.

## Detalles importantes (aprendidos midiendo)

- **El callback del atajo no hace trabajo**: encola `toggle` en el hilo principal (`run_on_main_thread`) y vuelve. Si un hook de bajo nivel tarda más de `LowLevelHooksTimeout`, Windows lo quita en silencio.
- **Raw Input de tao anula el hook**. Por defecto, tao registra el teclado como Raw Input en la ventana. Con ese registro activo, Windows no llamaba a nuestro hook de bajo nivel en este proceso: 0 llamadas medidas con un contador. El mismo código fuera de Tauri sí funcionaba. Solución: `.device_event_filter(DeviceEventFilter::Always)`. En tao, `Always` significa "filtrar siempre" y quita el registro. No usamos esos eventos, así que también ahorra CPU.
- **Foco**. Windows no deja a cualquier proceso traer su ventana al frente. Con `RegisterHotKey` el sistema nos da permiso; con el hook, no siempre. Al mostrar, si no estamos delante, se une un momento nuestra cola de entrada a la de la ventana activa (`AttachThreadInput`) antes de `SetForegroundWindow`.
- **Cambio en marcha**. Para cambiar el atajo, primero se suelta el anterior; si el nuevo falla, se vuelve a registrar el anterior. Nunca te quedas sin atajo.

## Consecuencias

- Es el único sitio con `unsafe` del proyecto. Está aislado en un módulo con `#[allow(unsafe_code)]` y cada bloque explica por qué es correcto.
- Limitación conocida del hook: no ve las teclas cuando la ventana activa es de un proceso elevado (administrador), por UIPI. `RegisterHotKey` no tiene esa limitación, y por eso es la vía principal.
- Medido en Windows 10 (build de debug): del atajo al primer fotograma pintado, 32–41 ms.
