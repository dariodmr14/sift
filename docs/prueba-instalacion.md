# Pruebas antes de publicar la 1.0: Windows Sandbox y una persona

Dos pruebas que hace Darío antes de publicar:
1. **Windows Sandbox:** que el instalador funciona en un Windows **recién estrenado**, sin nada de lo que tiene tu PC.
2. **Una persona:** que alguien que **no conoce Sift** lo instala y lo usa sin explicaciones.

Lo que salga de cada una va a `FEEDBACK.md`, con fecha, como siempre.

---

## 1. Windows Sandbox

Windows Sandbox es un Windows de usar y tirar dentro de tu PC. Arranca limpio, y al cerrarlo se borra todo lo que se hizo dentro. Viene con Windows 10 Pro.

### Activarlo (una vez)

1. Inicio → «Activar o desactivar las características de Windows».
2. Marca **«Espacio aislado de Windows»** y pulsa Aceptar.
3. Reinicia cuando lo pida.

Si la casilla está gris, falta la virtualización en la BIOS (Intel VT-x o AMD-V).

### Abrirlo con Sift dentro

1. Genera el instalador si no lo tienes: `scripts/instalar.ps1 -SoloCompilar`. Queda en `target\release\bundle\nsis\Sift_<versión>_x64-setup.exe`.
2. Desde la raíz del repo:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/sandbox.ps1
   ```

En el Escritorio del Sandbox aparecen tres carpetas, las tres en solo lectura:
- **«Instalador de Sift»** (se abre sola).
- **«Datos de prueba»**: los archivos inventados de `demo/`.
- **«Pruebas de seguridad»**: dos Espacios para importar.

Si algo falla, los registros están en `%LOCALAPPDATA%\dev.sift.app\logs` **dentro del Sandbox**. Cópialos **antes de cerrarlo**: al cerrarlo se pierden.

### A. Instalación y primer arranque

| # | Paso | Qué tiene que pasar |
|---|---|---|
| A1 | Doble clic en `Sift_…_x64-setup.exe` | SmartScreen avisa de que no está firmado: «Más información» → «Ejecutar de todas formas». **No** pide permisos de administrador |
| A2 | El asistente | En español, con la imagen del tamiz a la izquierda y «Te damos la bienvenida a Sift». En las páginas de dentro, el tamiz arriba a la derecha |
| A3 | Terminar con «Abrir Sift» marcado | Sift se abre y sale la **introducción** (paso 1 de 5), con su ilustración |
| A4 | Inicio | Sift en el menú Inicio con el icono del tamiz. En la bandeja, el tamiz en blanco |
| A5 | Pulsar **Alt+Espacio** (haz clic antes dentro del Sandbox) | Se abre la barra y la introducción marca ✓ |
| A6 | «Elegir otra carpeta…» → Escritorio → **Datos de prueba** | Mientras lee, el tamiz se mueve junto a «Leyendo…»; acaba en «Al día · 12 archivos» |
| A7 | En el paso 3, buscar «pago» | Salen `contrato-servicios.pdf` (con «Pág. 1»), `presupuesto.txt` y `notas-reunion-cliente.md` |
| A8 | Búsqueda inteligente → «Descargar» | El tamiz y «Descargando X de 135 MB»; acaba en «Lista». También vale «Ahora no» |
| A9 | Crear el primer Espacio y «Empezar» | Se cierra la ventana y sale la barra |

### B. Uso

| # | Paso | Qué tiene que pasar |
|---|---|---|
| B1 | Barra: «información duplicada» | Sale `tema-3-normalizacion.md` con «Por significado» (si la búsqueda inteligente está lista) |
| B2 | Tab sobre ese resultado | Vista previa con «Trata de lo mismo» y el pasaje marcado |
| B3 | «pdf aislamiento» | La etiqueta PDF y `apuntes-transacciones.pdf` con «Pág. 2» |
| B4 | Ajustes → Espacios → nuevo Espacio: carpeta «Datos de prueba» en la mitad izquierda | Al abrirlo desde la barra, el panel «Abriendo…» con el tamiz y la carpeta en su mitad |
| B5 | En el editor, «Identificar» | Un número grande en la pantalla del Sandbox durante 3 s |
| B6 | Ajustes → Estado | Tarjetas en verde, o diciendo qué falta |

### C. Seguridad

| # | Paso | Qué tiene que pasar |
|---|---|---|
| C1 | Ajustes → Espacios → Importar… → **Pruebas de seguridad** → `espacio-trampa.json` | Se **rechaza**: «Apuntes (trampa)… no es una carpeta». No se importa nada y no se abre la calculadora |
| C2 | Importar `espacio-app-por-ruta.json` | Se importa «Calculadora por ruta» |
| C3 | Abrir «Calculadora por ruta» desde la barra | **Antes** de abrir nada, un diálogo de Windows: «… va a ejecutar un programa o comando por primera vez: C:\Windows\System32\calc.exe». Con «Cancelar» no se abre nada; con «Ejecutar» se abre la calculadora, y la siguiente vez ya no pregunta |

### D. Actualizar y desinstalar

| # | Paso | Qué tiene que pasar |
|---|---|---|
| D1 | Con Sift instalado y con datos, volver a ejecutar el instalador | «Ya está instalado» → Siguiente → termina. Tus carpetas, Espacios y ajustes siguen ahí |
| D2 | Configuración de Windows → Aplicaciones → Sift → Desinstalar | Se va del menú Inicio y de la bandeja. Sin marcar «Eliminar los datos de aplicación», los datos se quedan en `%APPDATA%\dev.sift.app` |

---

## 2. Prueba con una persona que no conozca Sift

**Objetivo:** comprobar que alguien lo instala y lo usa **sin explicaciones**. No se prueba a la persona: se prueba Sift.

### Preparación (10 min)

- Un PC con Windows 10 u 11. Vale el Sandbox de arriba, con «Datos de prueba».
- El instalador en el Escritorio y Sift sin instalar.
- Un documento para apuntar y un reloj.
- Si la persona está de acuerdo, graba la pantalla (Win+G) y su voz. Dile para qué es y que lo borrarás después.

### Lo que le dices (y nada más)

> «Esto es Sift, un programa para encontrar archivos y abrir de golpe lo que usas para una tarea. Instálalo y haz lo que te vaya pidiendo. Piensa en voz alta: di qué esperas que pase y qué te confunde. Si te atascas, dímelo, pero intenta seguir solo. Estamos probando el programa, no a ti: si algo no sale, es culpa de Sift.»

Mientras tanto:
- **No expliques nada ni señales la pantalla.** Si pregunta «¿y ahora qué?», contesta «¿qué crees que harías?».
- **Ayuda solo si lleva más de 2 minutos atascada** en una tarea, y apunta exactamente qué le dijiste: eso es lo que hay que arreglar.

### Tareas (de una en una, sin decir cómo)

| # | Lo que le pides | Qué se está probando |
|---|---|---|
| 1 | «Instala Sift y ábrelo.» | Instalador y primer arranque |
| 2 | «Haz que Sift busque en la carpeta "Datos de prueba" del Escritorio.» | Añadir carpetas (introducción) |
| 3 | «Encuentra el presupuesto de la tienda online.» | La barra y el atajo |
| 4 | «Encuentra los apuntes que hablan de no guardar la misma información dos veces.» | La búsqueda por significado: los apuntes no usan esas palabras |
| 5 | «Mira qué dice ese archivo sin abrirlo.» | La vista previa (Tab) |
| 6 | «Encuentra el PDF que habla de transacciones y ve a la página donde sale "aislamiento".» | Los filtros («pdf …») y «Pág. X» |
| 7 | «Crea algo que, de una vez, abra la carpeta de la tienda a la izquierda de la pantalla y el presupuesto a la derecha.» | Los Espacios y colocar ventanas |
| 8 | «Ábrelo.» | Abrir un Espacio desde la barra |
| 9 | «Cierra Sift y vuelve a abrir la barra desde cualquier sitio.» | El atajo, otra vez |

### Qué apuntar en cada tarea

| Tarea | ¿Lo consiguió? (sí / con una pista / no) | Tiempo | Dónde dudó, qué dijo, qué pista le diste |
|---|---|---|---|
| 1 … 9 | | | |

También conviene apuntar:
- lo que dice en voz alta cuando algo le sorprende («¿y esto?», «ah, vale»);
- dónde busca primero algo que no encuentra;
- los textos que lee dos veces o que se salta.

### Al final (5–10 min)

1. ¿Qué crees que hace Sift, en una frase?
2. ¿Qué fue lo más confuso?
3. ¿Qué te ha sorprendido para bien?
4. ¿Lo usarías? ¿Para qué, en tu día a día?
5. Del 1 al 5, ¿cómo de fácil ha sido?
6. ¿Te has fiado de lo que hacía con tus archivos? ¿Qué te daría más confianza?

### Cuándo se da por buena

- Tareas 1, 2, 3, 8 y 9 **sin ayuda**.
- Tareas 4, 5, 6 y 7 **sin ayuda o con una pista**.
- La respuesta 1 se parece a «encuentra archivos (también por lo que dicen) y abre cosas de golpe».
- Ninguna tarea acaba con un error que no se entiende.

Si no se cumple, lo que falló va a `FEEDBACK.md` y se arregla antes de publicar.
