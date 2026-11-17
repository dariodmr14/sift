# Lo aprendido

Lo más útil de cada fase de Sift, contado al terminarla. El resumen está en el [README](../README.md#cómo-se-ha-construido).

**Fase 1**
- Mantener la ventana creada y oculta es lo que hace posible el < 100 ms. Una vez hecho eso, el tiempo restante (~33 ms) es esperar a que Windows componga unos dos fotogramas, y el código propio apenas cuenta: debug y release miden casi igual.
- Un hook de teclado de bajo nivel puede no recibir **nada** sin dar ningún error. La causa era que tao (la capa de ventanas de Tauri) registra el teclado como Raw Input. Se encontró aislando variables: el mismo código en un ejecutable mínimo funcionaba; con Tauri, el contador de llamadas marcaba 0.
- Windows protege el foco: no cualquier proceso puede traer su ventana al frente. Por eso funciona `AttachThreadInput`.
- Generar los tipos de TypeScript desde Rust (tauri-specta), con un test que falla si se desincronizan, convierte los errores de IPC en errores de compilación.

**Fase 1.5**
- La seguridad de los comandos tiene que imponerla Rust, no la UI. `run_space` contesta «hace falta confirmar» sin lanzar nada, y la aprobación va ligada al comando exacto: si cambia un argumento, es otro comando.
- Separar la lógica de *cómo* se lanza en Windows (el trait `Opener`) permite probar con un lanzador falso que un argumento como `a & del C:\*` llega entero, como uno solo.
- En Windows, crear una ventana WebView2 desde un comando IPC síncrono se bloquea y la ventana se queda en blanco: tiene que ser `async`.
- Un sistema de diseño solo se mantiene si algo lo vigila: un test que falla con colores literales vale más que una guía.
- Probar sin poder hacer clic: la depuración remota de WebView2 permite manejar la app real por el protocolo de DevTools (teclas, IPC y capturas).

**Fase 2**
- Un stemmer arregla los plurales pero rompe las erratas: «redundansia» y «redundancia» dan raíces distintas. La solución fue indexar cada campo dos veces, uno con raíces y otro solo sin tildes, y buscar en los dos.
- Diseñar desde el principio para lo que viene (fragmentos como unidad, un trait `Retriever` y fusión RRF) hace que la Fase 4 sea añadir un recuperador, no rehacer la búsqueda.
- SQLite es la fuente de verdad y Tantivy se puede reconstruir desde ella: un índice roto o de otro formato no pierde nada.
- Para decir «Leyendo 32 de 120» hay que separar *buscar cambios* (solo tamaño y fecha, barato) de *leer*. Así, además, se lee primero lo último que has tocado.
- Acrylic va a tirones al mover una ventana en Windows 10, pero solo mientras se mueve: cambiar a blur durante el arrastre da el mismo vidrio en reposo sin pagar el tirón.
- Para el arrastre con animación, una librería probada (dnd-kit) gana a un hook propio: medir, animar a los vecinos y la accesibilidad ya vienen resueltos.

**Fase 3**
- Indexar *todo* era el error: de 86.509 archivos de una carpeta real de la universidad solo 11.888 servían para buscar. Una lista de formatos permitidos y reconocer las carpetas generadas por lo que las rodea (la `Library` de Unity junto a `Assets`) dejó el índice en un 14 % y la base de datos de 620 MB en una fracción.
- Un renombrado llega del sistema como «desaparece A, aparece B». Casarlos por el hash del contenido conserva el id del archivo (sus fragmentos y su historial) y evita leerlo otra vez.
- Para recuperarse de un cierre a mitad basta con una marca por archivo: SQLite se guarda archivo a archivo y el índice de búsqueda por lotes, así que lo que quede sin confirmar se reenvía desde SQLite al arrancar, sin tocar el disco.
- Que el indexado «no se note» tiene tres capas: el modo de segundo plano de Windows (que también baja la prioridad del disco), un descanso proporcional al trabajo y una pausa con histéresis cuando el sistema está ocupado.
- Los materiales de Windows no se pueden fundir entre sí, pero sí taparse: un velo del color del fondo que entra antes de cambiar y sale después esconde el salto.

**Fase 4**
- Elegir el modelo midiendo: en las tablas públicas Granite R2 (mayo de 2026) superaba a e5-small por 9 puntos; con mis consultas en español, e5-small salió mejor en todo y, además, el doble de rápido.
- Una búsqueda de palabras que exige todas las palabras no encuentra nada con una frase («cómo evitar… en tablas»): por eso la semántica es lo que salva las ideas. Pero también por eso la semántica sola no basta para nombres y códigos (0,20): la fusión decide por tipo de búsqueda.
- Los pesos y umbrales «a ojo» del diseño estaban mal: el umbral de parecido no ayudaba (la protección real es el tipo de búsqueda) y un tercer recuperador que parecía obvio (BM25 con alguna de las palabras) no mejoraba nada. `eval/` lo dijo en minutos.
- Lo caro de una búsqueda no era el modelo (4 ms) sino cargar cien documentos de Tantivy para enseñar diez: leer solo ids de las columnas rápidas y cargar el texto al final bajó la p95 de 70 a 37 ms.
- Un vector por **texto** (hash) y no por fragmento: el 40 % de los fragmentos estaban repetidos, y reindexar un archivo no tira sus vectores.
- usearch (C++) abre las rutas con la codificación ANSI de Windows: con `C:\Users\Darío` no guardaba el índice. Los tests no lo vieron porque la carpeta temporal se llama `DARO~1`; ahora hay uno con tildes.

**Fase 5**
- Diagnosticar con datos y no con intuición: la sospecha era el modelo, y la causa grande resultó otra. La búsqueda de palabras devolvía *cero* resultados con frases como «cómo evitar redundancia en tablas» (ningún fragmento tiene todas las palabras), así que la fusión solo veía la semántica. Admitir que falte una palabra subió el total de 0,65 a 0,75.
- Lo que en la Fase 4 se descartó («alguna de las palabras») y lo que ahora funcionó se parecen, pero no son lo mismo: un recuperador aparte de BM25 puro mete ruido; la misma búsqueda exigiendo casi todas las palabras, detrás de las completas, no.
- Un índice aproximado (HNSW) se equivoca más de lo que parece: con el esfuerzo por defecto perdía el 12 % de los vecinos de verdad. Medirlo contra la búsqueda exacta costó minutos.
- Un benchmark por piezas encuentra lo que no esperas: lo más lento de una búsqueda no era el modelo ni el índice, sino **resaltar** el texto (se creaba un analizador por palabra). Arreglarlo bajó la híbrida a la mitad.
- Cuando no hay datos reales (4 aperturas), se puede calibrar simulando, siempre que el caso que ayuda y el que molesta se midan a la vez.
- Una métrica que baila ±0,02 entre ejecuciones no sirve para decidir: el baile venía de empates resueltos por el orden interno del índice.

**Fase 6**
- Comparar cosas de escalas distintas (Espacios por nombre, archivos por relevancia) se resolvió poniendo los Espacios en la escala de los archivos: «como el mejor archivo × lo bien que encaja × su frecencia». Una regla que se explica en una frase y se prueba con tests.
- Mezclar en la UI y no en Rust: los Espacios se filtran al instante en TypeScript y los archivos llegan después. Rust solo manda lo necesario para comparar (la frecencia de cada uno).
- Para que una espera se pruebe en milisegundos, el reloj y el «dormir» van detrás de un trait: en los tests, esperar 60 s es sumar 60 a un número.
- Un detalle que lo cambia todo: los comandos se abrían en su propia consola, que se cierra al terminar. Para poder decir por qué falló uno, el comando que se espera se lanza sin ventana y se lee su salida mientras se produce (si nadie la lee, el programa se bloquea).
- «Solo cosas de tu equipo» hay que comprobarlo por el host exacto, sin seguir redirecciones ni usar proxies: `localhost.ejemplo.com` también empieza por «localhost».

**Fase 6.5**
- Las apps más usadas (VS Code, navegadores, el Explorador) no abren un proceso nuevo: le piden al que ya tienen que abra otra ventana. Seguir el proceso lanzado no sirve; lo que funciona es buscar una ventana **nueva** del programa esperado.
- Windows 10 rodea las ventanas de un borde invisible de unos 7 px: si se mueve la ventana a la zona tal cual, quedan huecos. Hay que preguntar a DWM qué parte se ve y compensar.
- Probar ventanas sin tocar el ratón: una prueba abre ventanas de verdad, las coloca y mide dónde quedan. Para la interfaz, el protocolo de DevTools del WebView permite manejar la app real por script y hacer capturas.

**Fase 7**
- unslop dio un método: primero una dirección escrita (carácter, momentos y firma) y luego pasadas por orden (espaciado, color, iconos, textos, tipo y detalles). Con una dirección aprobada, cada pantalla se decide sola.
- La firma, «la marca de lo que encaja» (un subrayado de 2 px con el acento legible), une la lista, la vista previa y la búsqueda por significado con una sola idea visual.
- El acento de Windows no garantiza contraste. Rust calcula una variante que cumple 4,5:1 (`--accent-ink`) para el texto y los trazos; el acento sin cambios queda para los rellenos.
- Accesibilidad que se prueba: diálogos que atrapan el foco, anuncios con `aria-live` y objetivos de 24 px. El script de capturas lo comprueba con teclas reales por el protocolo de DevTools.
- Pedir permiso antes de la única descarga grande (el modelo, ~135 MB) en la bienvenida, en vez de descargar en silencio. La decisión y sus alternativas están en el [ADR 0015](adr/0015-bienvenida-y-permiso-de-descarga.md).
- La vista previa sale de SQLite (los fragmentos ya guardados), no del archivo: no lee disco y no afecta a la apertura de la barra ni a la búsqueda.
- Verla en pantalla cambió decisiones: el Markdown en un solo párrafo era ilegible, y se pasó a conservar los saltos de línea (salvo en PDF). Los tests no lo habrían visto.
- Las pruebas de extremo a extremo destapan supuestos: la barra solo busca al cambiar el texto, así que en el primer indexado la prueba fallaba. Ahora repite la búsqueda hasta que hay resultados.
- Con Enter por el protocolo de DevTools hay que mandar el carácter `\r`: sin él, el botón no se activa.
