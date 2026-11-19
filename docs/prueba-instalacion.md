# Prueba de instalación limpia y prueba con una persona

Dos pruebas antes de publicar: que el instalador funciona en un Windows **recién estrenado** (sin nada de lo que tiene tu PC), y que alguien que **no conoce Sift** lo usa sin explicaciones. La segunda es la que cierra la Fase 7.

## 1. Instalación limpia en Windows Sandbox

Windows Sandbox es un Windows de usar y tirar dentro de tu PC: arranca limpio, y al cerrarlo se borra todo lo que se hizo dentro. Viene con Windows 10 Pro.

### Activarlo (una vez, Darío)
1. Inicio → «Activar o desactivar las características de Windows».
2. Marca **«Espacio aislado de Windows»** y Aceptar.
3. Reinicia cuando lo pida.

Si la casilla está gris, falta la virtualización en la BIOS (Intel VT-x o AMD-V). Se activa en la BIOS del equipo.

### Abrirlo con Sift dentro
1. Genera el instalador, si no lo tienes: `scripts/instalar.ps1` o, en `apps/desktop`, `npm run tauri build`. Queda en `target\release\bundle\nsis\Sift_<versión>_x64-setup.exe`.
2. Desde la raíz del repo:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/sandbox.ps1
   ```

   Abre el Sandbox con dos carpetas en su Escritorio, las dos en solo lectura: **«Instalador de Sift»** (se abre sola) y **«Datos de prueba»** (los archivos inventados de `demo/`, para tener algo que buscar).

### Qué comprobar dentro

| # | Paso | Qué tiene que pasar |
|---|---|---|
| 1 | Doble clic en `Sift_…_x64-setup.exe` | Windows avisa de que no está firmado (SmartScreen): «Más información» → «Ejecutar de todas formas». El instalador no pide permisos de administrador |
| 2 | Terminar la instalación | Sift aparece en el menú Inicio con el icono del tamiz. Si falta WebView2, el instalador lo descarga (el Sandbox tiene internet) |
| 3 | Abrir Sift | Sale la **introducción** (paso 1 de 5), no la barra |
| 4 | Pulsar **Alt+Espacio** con la ventana del Sandbox activa | Se abre la barra y la introducción marca ✓. Si no, que Windows (el de fuera) no se esté quedando el atajo: haz clic dentro del Sandbox primero |
| 5 | «Elegir otra carpeta…» → Escritorio → **Datos de prueba** | Se añade y pasa a «Al día · 12 archivos» en unos segundos |
| 6 | Buscar «pago» en el paso 3 | Sale `contrato-servicios.pdf` (con «Pág. 1»), `presupuesto.txt` y `notas-reunion-cliente.md` |
| 7 | Búsqueda inteligente → «Descargar» | Descarga unos 135 MB y pasa a «Lista». También vale «Ahora no» |
| 8 | Crear el primer Espacio y «Empezar» | Se cierra la ventana y sale la barra |
| 9 | Bandeja del sistema | El tamiz en blanco (la barra de tareas del Sandbox es oscura) |
| 10 | Ajustes → Estado | Las tarjetas en verde, o diciendo qué falta |

Si algo falla: los registros están en `%LOCALAPPDATA%\dev.sift.app\logs` **dentro del Sandbox**. Ábrelos o cópialos **antes de cerrarlo**: al cerrarlo se pierden.

## 2. Prueba con una persona que no conozca Sift

**Objetivo:** comprobar el criterio de la Fase 7, que alguien lo instala y lo usa **sin explicaciones**. No se prueba a la persona, se prueba Sift.

### Preparación (5 min)
- Un PC con Windows 10 u 11. Puede ser el Sandbox de arriba, con «Datos de prueba».
- El instalador en el Escritorio. Sift sin instalar.
- Un papel o un documento para apuntar. Si la persona está de acuerdo, graba la pantalla (Win+G).

### Lo que le dices (y nada más)
> «Esto es Sift, un programa para encontrar archivos y abrir de golpe lo que usas para una tarea. Instálalo y haz lo que te pida. Piensa en voz alta: di qué esperas que pase y qué te confunde. Si te atascas, dímelo, pero intenta seguir solo. Estamos probando el programa, no a ti.»

### Tareas (de una en una, sin decir cómo)
1. **Instala Sift y ábrelo.**
2. **Haz que Sift busque en la carpeta «Datos de prueba» del Escritorio.**
3. **Encuentra el presupuesto de la tienda online.**
4. **Encuentra los apuntes que hablan de no guardar la misma información dos veces.** Es la búsqueda por significado: los apuntes no usan esas palabras.
5. **Mira qué dice ese archivo sin abrirlo.** Es la vista previa.
6. **Crea algo que abra de una vez la carpeta de la tienda y el presupuesto.** Es un Espacio.
7. **Cierra Sift y vuelve a abrirlo desde cualquier sitio.** Es el atajo.

### Qué apuntar en cada tarea

| Tarea | ¿Lo consiguió? (sí / con ayuda / no) | Tiempo | Dónde dudó o qué dijo |
|---|---|---|---|
| 1 … 7 | | | |

Ayuda solo si lleva más de 2 minutos atascado, y apunta qué tuviste que decir: eso es lo que hay que arreglar.

### Al final (5 min)
1. ¿Qué crees que hace Sift, en una frase?
2. ¿Qué fue lo más confuso?
3. ¿Qué te ha sorprendido para bien?
4. ¿Lo usarías? ¿Para qué?
5. Del 1 al 5, ¿cómo de fácil ha sido?

### Cuándo se da por buena (Fase 7)
- Tareas 1, 2, 3 y 7 **sin ayuda**.
- Tareas 4, 5 y 6 **sin ayuda o con una pista**.
- La respuesta 1 se parece a «encuentra archivos y abre cosas de golpe».

Lo que salga va a `FEEDBACK.md`, con fecha, como siempre.
