# Pendiente de decidir

Lo que salió trabajando de noche (2026-11-17/18) y necesita tu visto bueno. Cada punto tiene contexto, opciones y mi recomendación. Los marcados **decidido de noche** ya están hechos así porque eran pequeños y reversibles: si no te convencen, se cambian en un momento.

## Vista previa (bloque 7)

### 1. Tab sobre un Espacio o un reciente — decidido de noche
- **Contexto:** la vista previa enseña el texto guardado de un archivo encontrado. Los Espacios y lo «Abierto recientemente» no tienen fragmento.
- **Opciones:** (a) Tab no hace nada con ellos; (b) para un Espacio, enseñar su lista de acciones; (c) para un reciente, enseñar el principio del archivo.
- **Hecho:** (a). Si con la vista previa abierta bajas con ↓ hasta un Espacio, el panel dice «Este resultado no tiene vista previa.» en vez de cerrarse.
- **Recomendación para más adelante:** (b) es bonito y barato; al backlog.
- **Aprobado por Darío (2026-11-18), con cambio:** Tab sobre un Espacio enseña sus acciones en orden y Enter lo abre (hecho en `feat/vista-previa-espacios`).

### 2. Tamaño del código en la vista previa — decidido de noche
- **Contexto:** el texto normal va a 14 px; el código, en fuente fija, a 14 px no cabe en el ancho de la barra sin romper líneas.
- **Hecho:** código a 12 px (`text-xs`, de la escala de Vidrio), con sus saltos de línea y la indentación.

## Comprobación en pantalla (tarea 2)

### 3. Ver la bienvenida en la versión de desarrollo — decidido de noche
- **Contexto:** la versión de desarrollo se rellena sola con datos de prueba la primera vez, y con datos la bienvenida no sale (solo sale en un equipo nuevo). Renombrar la carpeta de datos no basta.
- **Hecho:** una variable solo para desarrollo, `SIFT_DEV_NO_SEED=1`, que arranca sin datos de prueba. En la versión diaria no hace nada: los datos de prueba solo se crean en las compilaciones de desarrollo.

### 4. Saltos de línea en la vista previa — decidido de noche
- **Contexto:** al verla en pantalla, la vista previa de un Markdown o un texto juntaba todo en un párrafo; las listas y tablas quedaban ilegibles («| --- | --- |»).
- **Hecho:** se conservan los saltos de línea en todo **salvo en los PDF**, cuyo texto viene cortado por la maquetación de la página y se lee mejor seguido. Sin barra de desplazamiento: el fragmento ya viene recortado (12 líneas) y el foco está en la caja.
- **Captura:** `docs/capturas/final/vista-previa.jpg`.

### 5. La barra no se refresca sola mientras se indexa — al backlog
- **Contexto:** lo destapó la prueba de extremo a extremo. Si buscas mientras Sift lee tus carpetas por primera vez, los resultados no se actualizan hasta que cambias el texto.
- **Opciones:** (a) dejarlo (es solo el primer indexado); (b) volver a buscar cuando el estado pasa a «Al día», si la barra está abierta.
- **Recomendación:** (b), es barato. Lo he dejado en el backlog de `PLAN.md`.

## Búsqueda

### 6. Copias del mismo archivo — decidido de noche
- **Contexto:** el mismo apunte copiado en varias carpetas salía varias veces seguido (`clarin.txt` ×3) y ocupaba puestos de otros archivos.
- **Hecho:** si dos resultados tienen el mismo nombre (sin mayúsculas ni tildes) y el mismo fragmento, solo sale el mejor colocado. Lo que no tiene texto (fotos, vídeos) no se agrupa nunca, porque dos `IMG_0001.jpg` suelen ser fotos distintas. Medido con `eval/`: recall@10 y MRR iguales, y las copias repetidas en el top 10 de las 34 consultas bajan de 39 a 12.
- **Pendiente de ti:** las otras copias ya no se ven. Opciones: (a) dejarlo así (cualquiera vale); (b) decir «y 2 copias más» en el subtítulo, con Ctrl+↵ abriendo la carpeta de la que sale. **Recomendación:** (b) si alguna vez te ha importado cuál de las copias abrías; si no, (a).
- **Las 12 que quedan** tienen el mismo nombre con distinto texto: versiones distintas de un apunte o un fragmento mejor colocado distinto. Agruparlas solo por nombre escondería versiones distintas, así que no lo he hecho.

## Tests y cobertura

### 7. Cobertura de Rust — necesita tu permiso
- **Contexto:** medir la cobertura de Rust exige instalar `llvm-tools-preview` (con rustup) y `cargo-llvm-cov` (con cargo install). Las dos cosas van fuera del proyecto y, según tus reglas, no las instalo yo solo.
- **Aprobado y hecho (2026-11-18):** medido en `docs/cobertura.md`.
- **Si te parece bien:** `rustup component add llvm-tools-preview`, `cargo install cargo-llvm-cov --locked` y `cargo llvm-cov --workspace --summary-only`. Mientras tanto, `docs/cobertura.md` recoge, revisados a mano, los módulos que no tienen tests. El más crítico, `index.rs`, ya los tiene.

### 8. La ayuda iba dentro de la etiqueta de los campos — hecho de noche
- **Contexto:** lo destaparon los tests. `Field` (`settings/ui.tsx`) mete el texto de ayuda dentro de la `<label>`, y un lector de pantalla lee la ayuda entera como si fuera el nombre del campo.
- **Hecho:** la etiqueta lleva solo el nombre y la ayuda va con `aria-describedby`. En pantalla no cambia nada. Lo prueba `settings/ui.test.tsx`.

### 9. Avisos de `npm audit` en las herramientas de desarrollo
- **Contexto:** `npm audit` completo avisa de un fallo moderado en `minimatch`, que llega por eslint y typescript-eslint. **No afecta a la app:** la CI solo audita las dependencias de producción, y están limpias.
- **Recomendación:** actualizar eslint y typescript-eslint cuando salga una versión con el arreglo, en un PR aparte. No es urgente.
- **Hecho (2026-11-18):** eslint 10.15.0 y typescript-eslint 8.73.0 quitan el de `minimatch`. Ha salido otro, alto, en `picomatch` (ReDoS), que llega por vite, vitest y typescript-eslint: todas sus versiones están afectadas y aún no hay arreglo (lo que propone `npm audit fix --force` es bajar vitest a la 3, que no arregla nada útil). Solo afecta a herramientas de desarrollo con patrones que escribimos nosotros; la app tiene 0 avisos. Se revisa cuando salga la versión arreglada.

## Pendiente de revisar

### 10. Aviso de `picomatch` en `npm audit` — esperando el arreglo
- **Qué es:** un fallo alto de `picomatch` (ReDoS, GHSA-fc5q-39xc-vjpf) que llega por vite, vitest y typescript-eslint.
- **Por qué no se arregla ya:** todas sus versiones están afectadas y no hay ninguna corregida. `npm audit fix --force` propone bajar vitest a la 3, y eso no lo soluciona.
- **A quién afecta:** solo a herramientas de desarrollo, con patrones que escribimos nosotros. La app tiene 0 avisos (`npm audit --omit=dev`, lo que comprueba la CI).
- **Qué hacer:** cuando salga una versión de `picomatch` arreglada, `npm update` en `apps/desktop/` y comprobar `npm audit`. Decidido con Darío el 2026-11-18: se deja apuntado hasta entonces.

## Auditoría 1.0 (2026-11-20, trabajando solo)

### 11. Qué se abre sin preguntar — decidido sin ti
- **Contexto:** pediste una lista de lo **permitido**, no de lo bloqueado. Ahora `SAFE_TO_OPEN` (`crates/spaces/src/validate.rs`) recoge documentos, imágenes, audio, vídeo, comprimidos, texto y el código que Windows no ejecuta al abrirlo. Todo lo demás, también una extensión que aún no existe, pide confirmación.
- **Hecho:** en un Espacio, un archivo fuera de la lista (o una app que no es un id de Windows, como `C:\…\x.exe` o `{GUID}\cmd.exe`) pide **el mismo diálogo nativo que un comando** la primera vez, y luego queda aprobado. En la barra, cada vez sale un diálogo de Windows «… puede ejecutar un programa al abrirlo» con Abrir / Cancelar (antes, la rc.3 enseñaba el archivo en su carpeta).
- **Lo que puede molestar:** los accesos directos (`.lnk`) del Escritorio que salgan en la búsqueda preguntan cada vez. **Opciones:** (a) dejarlo así (lo más seguro); (b) recordar la respuesta por archivo, como en los Espacios; (c) resolver el `.lnk` y abrir sin preguntar si apunta a una carpeta o a un documento de la lista. **Recomendación:** (c) para la 1.1.
- **Fuera de la lista a propósito:** `.js`, `.vbs`, `.py`, `.ps1`, `.bat` (Windows los ejecuta) y los documentos con macros (`.docm`, `.xlsm`…). Si echas de menos alguna extensión que sea segura, se añade en una línea.

### 12. La descarga del modelo, en su propio hilo — decidido sin ti
- **Contexto:** la descarga iba en el hilo del indexado: mientras bajaban los 135 MB, los archivos cambiados no se indexaban, y una conexión colgada lo paraba todo.
- **Hecho:** un hilo propio, con tiempos máximos (15 s para conectar, 30 s para la respuesta y 20 min para el cuerpo) y 3 intentos que siguen donde se quedó el anterior (a los 5 s y a los 30 s). Si fallan los tres, se vuelve a probar sola al cabo de 5 minutos, o antes si desactivas y vuelves a activar la búsqueda inteligente.

### 13. Todo en un PR en vez de tres — aviso
- **Contexto:** pediste un PR por grupo (seguridad, robustez e interfaz). La sesión anterior lo juntó todo en un único commit (`f21c572`) antes de cortarse, y separarlo ahora es reescribir el historial local.
- **Hecho:** va en un solo PR, con los commits de esta sesión separados por tema y la descripción dividida en los tres grupos.

## Preparar la publicación (2026-11-21, trabajando solo)

### 14. Limpieza antes de hacerlo público — necesita tu decisión
- **Contexto:** informe completo en [`docs/publicacion/limpieza.md`](../publicacion/limpieza.md). No hay secretos ni tu correo en ningún sitio; solo queda tu ruta de usuario (`C:\Users\Darío`) en un documento actual y en el historial.
- **Opciones y recomendación:**
  1. Cambiar la ruta en `docs/proceso/publicar-revision.md` por `C:\Users\<tu usuario>`: **sí** (un PR de cinco líneas).
  2. Borrar la rama remota `feat/instalador`, ya fusionada: **sí**.
  3. Instalar y pasar gitleaks: **opcional**. Al publicar, el escaneo de secretos de GitHub cubre lo mismo hacia adelante.
  4. Reescribir el historial: **no**. Obliga a un force push a `main` y rompe los enlaces de los 61 PR, por algo de gravedad baja.
