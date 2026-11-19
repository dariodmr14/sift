# ADR 0012 — Esperas en los Espacios (Fase 6)

- **Estado:** aceptada (propuesta aprobada por Darío con tres ajustes)
- **Fecha:** 2026-11-12

## Contexto

Un Espacio lanza sus acciones una tras otra, sin esperar. Para el entorno de una asignatura eso no basta. Por ejemplo: «arranca los contenedores y, cuando la web responda, ábrela». Si la web se abre antes de que el contenedor esté listo, sale una página de error.

## Decisión

### 1. Un paso más: «Esperar a que…»

Va entre las acciones, como cualquier otra, y se arrastra para colocarlo. Hay cuatro tipos, escritos en lenguaje normal:

| En el editor | Qué comprueba | Tiempo límite por defecto |
|---|---|---|
| …responda una web de tu equipo | Una petición a la dirección (p. ej. `http://localhost:8080`). Cualquier respuesta por debajo de 500 vale, también un 404, porque el servidor ya está en marcha. Un 5xx suele ser «arrancando». | 60 s |
| …un servicio de tu equipo acepte conexiones (como una base de datos) | Que `127.0.0.1` o `[::1]` acepten una conexión en el **puerto**. El puerto va en su propio campo, con una explicación y ejemplos (PostgreSQL 5432, MySQL 3306, Redis 6379). | 60 s |
| …termine el comando anterior | Que el comando de justo antes acabe sin error. | 120 s |
| …pasen unos segundos | Nada: solo espera. | — |

- Se comprueba cada medio segundo. El tiempo límite se cambia en el propio paso, entre 1 y 600 s.
- Debajo de cada espera hay una línea: **«Si no ocurre en [60] segundos: [no seguir y avisarme / seguir igualmente]»**. Por defecto, no seguir: abrir la web de un contenedor que no arrancó no sirve de nada.

### 2. Solo cosas de tu equipo

La web tiene que ser `localhost`, `127.0.0.1` o `[::1]`. Se valida el **host exacto**: `localhost.ejemplo.com` o `usuario@ejemplo.com` no valen. Al comprobarla:

- no se siguen redirecciones (podrían llevar fuera del equipo);
- no se usa el proxy del sistema (la petición iría a otro sitio).

Así se mantiene la regla «nada sale a la red» (solo se descarga el modelo).

### 3. El comando que se espera, sin ventana

Hoy cada comando se abre en su propia consola para que veas su salida, pero esa ventana se cierra al terminar. Si fallara, no verías por qué. Por eso, **solo el comando seguido de «Esperar a que termine»** se lanza sin ventana. Su salida se va leyendo mientras se produce (si nadie la lee, el comando se bloquea al llenarse el búfer) y se guardan sus últimas 40 líneas. Si acaba con error, la barra enseña el código y las 6 últimas. Los demás comandos se lanzan igual que antes.

«Esperar a que termine el comando anterior» tiene que ir **justo después** de un comando, y se valida al guardar.

### 4. Probar

El botón «Probar» de una espera de web o de servicio comprueba la condición **una vez, ahora**, y dice si se cumple, sin lanzar el Espacio. Las esperas de comando y de segundos no tienen nada que comprobar por adelantado: en ellas el botón está desactivado y explica por qué.

### 5. En la barra: progreso, cancelar y resumen

- Si un Espacio tarda más de 300 ms, la barra dice por qué paso va: «Abriendo «Bases de datos» · Paso 2 de 3 · Esperando a que responda http://localhost:8080… 12 s». Por debajo de ese tiempo no se enseña nada, para que no parpadee.
- **Esc cancela.** Lo que ya se abrió o lanzó sigue abierto: nunca se cierra nada. Al acabar, la barra lista cada paso:
  - ✓ hecho;
  - ✕ falló, con el motivo;
  - ✕ cancelado mientras esperaba;
  - – no se ejecutó.
- Si escondes la barra, el Espacio sigue. Si acaba con algo que contar, se ve al volver a abrirla.
- Un Espacio a la vez: si ya hay otro en marcha, se avisa.

### 6. Cómo está hecho

- `sift-spaces` tiene la lógica: qué se espera, cuándo se para y qué se cuenta. Lo que toca Windows queda detrás de tres traits: `Opener` (abrir y lanzar), `Waiter` (comprobar, dormir y el reloj) y `Child` (el comando que se espera). Los tests usan un reloj de mentira: esperar 60 s es instantáneo.
- La app pone los de verdad: `ureq` con 2 s de límite por comprobación, `TcpStream` con 400 ms, y el comando sin ventana. Emite un evento `SpaceProgress` por paso y por segundo de espera, y `cancel_space_run` levanta una marca que el bucle mira en cada vuelta.
- El modelo es una acción más, `Action::Wait(WaitSpec)`, guardada como JSON como las demás: no hace falta migrar la base de datos.

## Alternativas descartadas

- **Condiciones generales («solo si es lunes», «solo si existe esta carpeta»).** Lo que se necesitaba era esperar y decidir si seguir. Si aparece un uso real, se añaden.
- **Esperar a una web de internet.** Rompería la regla de no conectarse a la red.
- **Capturar la salida de todos los comandos.** Te quitaría su consola cuando sí la quieres ver, por ejemplo con un servidor de desarrollo.
- **Cerrar lo ya abierto al cancelar o al fallar.** Darío pidió explícitamente lo contrario. Además, cerrar procesos ajenos es arriesgado.

**Cambio (2026-11-19).** Un paso «Esperar a que…» es ahora lo único que hace esperar a un Espacio: colocar ventanas va en segundo plano (ADR 0014) y sigue funcionando mientras se espera. Al cancelar, lo que quedaba por colocar se cuenta como «cancelado antes de colocar su ventana».
