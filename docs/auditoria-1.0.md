# Auditoría 1.0

Revisión antes de publicar la 1.0 (2026-11-20). Los arreglos van en la 1.0.0-rc.4 (PR #57, #58, #59 y #60). Tiene tres partes: lo que bloqueaba la 1.0 y cómo se arregló, las cifras de las pruebas de aguante frente a los objetivos de `PLAN.md` y lo que queda para la 1.1.

## 1. Lo que bloqueaba la 1.0

### Seguridad: nada se ejecuta sin la aprobación nativa

| Problema | Arreglo | Test |
|---|---|---|
| Un Espacio (también uno importado de un JSON) podía abrir un `.exe` haciéndolo pasar por «carpeta». Windows lo ejecuta al abrirlo | Una carpeta tiene que ser una carpeta de verdad: se comprueba al guardar, al importar y otra vez al abrir. Tampoco valen las carpetas especiales de Windows (`x.{CLSID}`) | `a_folder_must_be_a_folder_and_paths_must_be_local`, `opening_a_folder_that_is_a_file_fails_at_run_time` |
| Una «aplicación» dada por la ruta de un programa (`C:\…\x.exe`, `{GUID}\cmd.exe`) se lanzaba sin preguntar | Solo se abre sin preguntar un **id de app de Windows** (letras, cifras y `. _ - !`). Cualquier otra cosa pide el mismo diálogo de Windows que un comando la primera vez | `an_app_given_by_a_program_path_asks_like_a_command`, `files_and_apps_outside_the_allowlist_ask_first` |
| «Abrir archivo» solo rechazaba 14 extensiones (`.exe`, `.bat`…). Se colaban `.lnk`, `.url`, `.hta`, `.reg`, `.cpl`, los documentos con macros… | **Lista de lo PERMITIDO** (`SAFE_TO_OPEN`): documentos, imágenes, audio, vídeo, comprimidos, texto y código que Windows no ejecuta. Todo lo demás, también lo desconocido, pide aprobación nativa. También se cierran los trucos de nombre que Windows ignora: punto o espacio al final y flujos alternativos de NTFS (`notas.txt:x.exe`) | `only_listed_extensions_open_without_asking` |
| Abrir un `.bat`, `.js` o `.lnk` desde un resultado de búsqueda lo ejecutaba | Lo que no está en la lista abre antes un diálogo de Windows («… puede ejecutar un programa al abrirlo», Abrir / Cancelar). Es un diálogo nativo, no de la interfaz: nada dentro del WebView puede aprobarlo | `open_file` en `search_commands.rs` (el diálogo se comprueba a mano, ver §2.4) |
| Rutas de red (`\\equipo\…`): abrirlas manda tus credenciales de Windows a ese equipo | Se rechazan: Sift solo abre rutas de este PC | `a_folder_must_be_a_folder_and_paths_must_be_local` |
| — | Los dos Espacios de la prueba en Sandbox (`scripts/sandbox-pruebas/`) se comprueban en un test, para que la guía no se quede desfasada | `the_sandbox_security_spaces_behave_as_the_guide_says` |

### Robustez: la barra y el atajo nunca se congelan

| Problema | Arreglo |
|---|---|
| 16 comandos de la interfaz que leen o escriben la base de datos eran síncronos: en Tauri corren en el **hilo principal**, el que atiende el atajo, y esperaban el candado de la base de datos mientras el indexador lo tenía | Pasan a `#[tauri::command(async)]`: corren en el grupo de hilos de Tauri, y el hilo principal queda libre |
| Al reconstruir el índice de vectores, el indexador tenía el candado de la base de datos todo el rato (segundos con muchos vectores) | El candado se tiene solo mientras se leen los vectores. Construir el índice y guardarlo va sin él |
| La descarga del modelo no tenía tiempo límite: una conexión colgada dejaba el indexado parado hasta reiniciar | Tiempos máximos (15 s para conectar, 30 s para la respuesta y 20 min para el cuerpo) y **3 intentos** que siguen donde se quedó el anterior (a los 5 s y a los 30 s). Si fallan, se vuelve a probar a los 5 minutos (test `network_failures_are_retried_and_disk_failures_are_not`) |
| La descarga iba **dentro del hilo del indexado**: mientras bajaban los 135 MB, los archivos nuevos o cambiados no se indexaban | Va en su propio hilo (`indexing/download.rs`). Al acabar avisa a la cola y el indexado carga el modelo (tests en `indexing::download`) |
| Con el PC al 15 %, el indexado se pasaba casi todo el rato en pausa: la carga se medía en ventanas de pocos milisegundos, y cualquier pico de otro programa pasaba del 75 % | La carga se mide sobre al menos 1 s (`load.rs`). Lo destapó la prueba de carga |
| **Memoria** (lo destapó la prueba de carga): con la búsqueda inteligente activa, Sift ocupaba **1,8 GB**. Había cuatro copias del tokenizador de e5, de unos 255 MB cada una (dos por sesión), y el «arena» de ONNX Runtime guardaba para siempre la memoria del lote más grande (+570 MB) | Un solo tokenizador por modelo para todo el proceso. El que cuenta tokens solo se crea en las pruebas. El arena de ONNX, desactivado (~15 % más lento al vectorizar en segundo plano; las consultas, igual). Medido con `crates/embeddings/examples/memoria.rs`: **1.847 → 515 MB** |
| **La barra esperaba hasta 4,8 s** mientras se vectorizaba (lo destapó la prueba de carga). Buscar qué faltaba por vectorizar recorría los 322 000 fragmentos leyendo su texto: **5,5 s cada 64 vectores**, con el candado de la base de datos. Además, el progreso tardaba 5,2 s | Migración v7: dos índices que cubren esas consultas sin leer el texto. Esas lecturas van por una **segunda conexión de solo lectura** (en modo WAL no esperan ni hacen esperar), y se piden 512 textos de una vez. Medido con `crates/storage/examples/vectores.rs` (100 000 archivos, 350 000 fragmentos): 5,5 s → 0,5 s cada 512 vectores, y sin candado. La migración tarda 3 s una sola vez con esa base de datos |
| Disco lleno | Es un error, no un cierre. No deja nada a medias y, cuando vuelve a haber sitio, se sigue guardando (test `disk_full_is_an_error_not_a_crash_and_the_database_stays_usable`) |

### Interfaz: nunca en blanco y nunca atascada

| Problema | Arreglo | Test |
|---|---|---|
| Si un componente fallaba, la pantalla se quedaba en blanco | `ErrorBoundary` en la barra y en Ajustes: un mensaje, «Volver a intentarlo» y Esc para cerrar | `ErrorBoundary.test.tsx` |
| Si la llamada para abrir un Espacio fallaba del todo, la barra se quedaba en «Abriendo…» | Se dice qué pasó y la barra se libera | `App.test.tsx`, «si la llamada para abrir un Espacio falla del todo…» |
| Un doble Enter (o doble clic) abría el Espacio dos veces | Mientras se abre uno, otro Enter no hace nada | `App.test.tsx`, «un doble Enter no abre el Espacio dos veces» |
| La barra se cerraba sola al abrirla justo después de cerrarla: la animación de cierre anterior acababa y la ocultaba. Además, cualquier animación de dentro (el tamiz) contaba como el final del cierre | El cierre pendiente se cancela al volver a abrir. Solo cuenta el final de la animación de salida de la propia barra, con un tope de 300 ms | `ErrorBoundary.test.tsx`, `dismissLauncher` |

## 2. Pruebas de aguante

Todas con la compilación de pruebas (perfil `aguante`, datos de desarrollo apartados y devueltos al acabar) y carpetas inventadas por `scripts/aguante/generar.py`. Los scripts están en `scripts/aguante/` y se manejan por las DevTools del WebView, sin tocar el teclado ni el ratón.

### 2.1 Carga: 100 000 archivos inventados

Carpeta de 266 MB: 100 000 archivos (documentos, apuntes, código, PDF, datos) que dan **322 639 fragmentos**, seis veces lo que pide el PLAN. Se hizo dos veces: antes de los arreglos de memoria y de vectores (1.ª) y después (2.ª).

| Medida | Objetivo (`PLAN.md`) | 1.ª | 2.ª |
|---|---|---|---|
| Búsqueda p95 (10 tipos de consulta × 25, vista desde la interfaz) | < 50 ms con ~50 000 fragmentos | 17,6 ms (p50 7,7) | **14,9 ms** (p50 7,4) |
| Búsqueda p95 medida en Rust | | 12,8 ms | 11,3 ms |
| Reposo, Sift | < 100 MB RAM | 70 MB (33 MB propios) | **55 MB** (27 MB propios) |
| Reposo, WebView2 | | 320 MB | 342 MB |
| Indexar 100 000 archivos desde cero | en segundo plano, sin molestar | 49 min | 51 min |
| Pico de memoria al indexar (Sift) | | 149 MB | 136 MB |
| Espacio en disco | | 609 MB | 619 MB (base de datos 436, índice 179) |
| Vectorizar (búsqueda inteligente) | en segundo plano | ~6 vectores/s | **~11 vectores/s** |
| Búsqueda p95 mientras vectoriza (peor muestra) | < 50 ms | **249 ms** | **37 ms** |
| Reiniciar con todo indexado: barra lista / primera búsqueda | | 1,6 s / 9 ms | 1,5 s / 10 ms |
| Tras reiniciar, buscar cada 250 ms durante 1 min | < 50 ms | una espera de **4,8 s** | 225 búsquedas, p95 30 ms, máx. 57 ms, ninguna > 100 ms |
| Memoria de Sift con la búsqueda inteligente | | **1 362 MB** | **602 MB** |

Qué dicen las cifras:
- La búsqueda cumple de sobra, también con seis veces más fragmentos de lo previsto.
- En reposo, **Sift** cumple (55 MB), pero el **WebView2** de Windows ocupa otros ~340 MB. Es el motor del navegador que usa Tauri y no depende de Sift. Queda para la 1.1 ver si se puede liberar con la barra cerrada.
- La búsqueda inteligente necesita unos 600 MB: el modelo y su tokenizador. Es opcional y pide permiso. Lo que no era aceptable eran los 1,8 GB y las esperas de la barra, y eso es lo que se arregló (§1).
- A ~11 vectores/s, vectorizar 322 000 fragmentos lleva unas 8 horas de PC encendido. Va en segundo plano y la búsqueda por palabras funciona mientras tanto, pero es lento (1.1).
- El atajo → ventana (< 100 ms) no lo miden estos scripts, porque necesita el teclado. Se midió en la Fase 1 y no se ha tocado.

### 2.2 Robustez

`scripts/aguante/robustez.mjs`. El disco lleno se prueba en Rust (§1).

**Cierres bruscos** (como un corte de luz, `taskkill /F`) indexando los 100 000 archivos, a los 25 s y a los 60 s:

| Medida | Resultado |
|---|---|
| Al volver a abrir | Sigue donde se quedó: solo lee lo que faltaba (97 862 de 100 000; los ~2 100 que ya había leído antes de los cierres no se repiten) |
| Hasta acabar | 50 min, sin errores (`error: null`) |
| Buscar «presupuesto» después | 12 resultados |
| Base de datos | Abre bien, sin reparar nada: el modo WAL de SQLite deshace lo que se quedó a medias |

**Archivos raros**, en una carpeta aparte:

| Archivo | Qué hace Sift |
|---|---|
| `enorme.txt` (600 MB) | Solo por nombre; en Ajustes → Carpetas, «demasiado grande» |
| `corrupto.pdf` (bytes al azar), `truncado.pdf` (medio PDF), `falso.docx` | «Dañados» (3), sin cerrarse ni atascarse |
| `binario.txt` (2 MB al azar con extensión de texto) | «No se puede leer» |
| `falso.xlsx`, `vacio.md` | Por nombre (de las hojas de cálculo no se lee el contenido) |
| `ñandú café — notas (copia) #1 😀.md` | Se encuentra buscando «presupuesto» |
| `nota-profunda.md` en una ruta de más de 300 caracteres | Se encuentra buscando «profunda» |
| Carpeta sin permiso de lectura | Se salta y el resto se indexa. Ahora queda en el registro |

Toda la carpeta, en 6 s, con 43 MB de memoria.

**Dos fallos encontrados y arreglados:**
- Tras poner al día unos cambios, el estado decía «0 de 7 leídos» en reposo. En la interfaz no se veía, pero la prueba se quedó esperando 2,5 h. Ahora un trabajo terminado deja `done = total` (test `a_finished_run_is_idle_with_everything_read`).
- La carpeta sin permiso se saltaba sin dejar rastro; ahora deja un aviso en el registro. Decirlo también en Ajustes queda para la 1.1.

### 2.3 Larga duración

`scripts/aguante/largo.mjs`: **3 horas** abierta con 10 000 archivos inventados y actividad sin parar. Una búsqueda escrita en la barra cada 3 s, archivos que se crean, cambian y se borran cada 20 s (el vigilante tiene que verlos) y un Espacio que se abre cada minuto.

| Medida | Resultado |
|---|---|
| Actividad | 3 501 búsquedas, 175 Espacios abiertos, ~540 cambios de archivos |
| Errores | **0** |
| Memoria de Sift | Entre 47 y 60 MB todo el rato (50 MB al acabar) |
| Memoria del WebView2 | Entre 319 y 354 MB (320 MB al acabar) |
| Pendiente de la memoria (sin los 10 primeros minutos) | Sift −3,1 MB/h; memoria propia de Sift +1,1 MB/h; WebView −14,6 MB/h. **No hay fugas** |
| Búsqueda p95 (medida en Rust), peor minuto | 13,7 ms |

No se ha probado la búsqueda inteligente durante horas: en la prueba de carga, la memoria se quedó estable mientras vectorizaba (§2.1).

### 2.4 Lo que tiene que comprobar una persona

- **Diálogo al abrir un archivo que puede ejecutar algo** (`.bat`, `.js`, `.lnk`… desde un resultado): el código y la lista tienen tests, pero que el diálogo nativo salga y que «Cancelar» no abra nada se comprueba a mano. Es el paso C4 de `docs/prueba-instalacion.md`.
- **Los Espacios trampa** de `scripts/sandbox-pruebas/`, importados en una Windows Sandbox (pasos C1–C3).
- **Una persona que no conoce Sift**, con la parte 2 de la misma guía.

## 3. Para la 1.1

No bloquean la 1.0, pero se notan. Por orden de lo que más se nota:

1. **Vectorizar más rápido.** ~11 vectores/s: con una carpeta muy grande, la búsqueda por significado tarda horas en cubrirlo todo. Opciones: lotes ordenados por longitud (menos relleno), DirectML (backlog) o vectorizar primero lo que más se abre.
2. **Memoria del WebView2 (~340 MB).** Es la mayor parte de lo que ocupa Sift en reposo. Probar a liberar la memoria del WebView con la barra oculta (`TrySuspend`/`MemoryUsageTargetLevel` de WebView2) y medir cuánto tarda en abrirse después: no puede pasar de 100 ms.
3. **Memoria de la búsqueda inteligente (~600 MB).** La mayor parte es el tokenizador de e5 (unos 255 MB, por su vocabulario de 250 000 piezas). Se podría cargar uno más pequeño (solo español e inglés) o liberar el modelo tras un rato sin buscar por significado.
4. **`.lnk` desde la búsqueda:** hoy pregunta siempre. Mejor seguir el acceso directo y decidir por su destino (decisión 11 de `pendiente-decidir.md`).
5. **Compartir la sesión de ONNX** entre el indexado y la barra: hoy hay dos (una para las consultas y otra para vectorizar).
6. **Arena de ONNX:** desactivarlo cuesta ~15 % al vectorizar. Volver a medir si se vectoriza con lotes de tamaño fijo, donde el arena no crece.
7. **Carpetas sin permiso:** decirlo en Ajustes → Carpetas, junto a «no se pudieron leer», en vez de solo en el registro.
8. Lo que sigue abierto en `FEEDBACK.md`: «por qué sale este resultado».
9. Del backlog de `PLAN.md`, lo que más pide el uso: volver a buscar sola cuando acaba el indexado y grabar el atajo pulsándolo.
