# ADR 0015 — Bienvenida y permiso para descargar el modelo

- **Estado:** aceptada
- **Fecha:** 2026-11-17

## Contexto

La primera vez que se abre Sift, la barra solo dice «Aún no tienes Espacios». No pide carpetas para buscar y, si se usa la búsqueda por significado, descarga unos 120 MB del modelo sin preguntar. La auditoría de la Fase 7 (F7, en [auditoria-fase-7.md](../auditoria-fase-7.md)) lo marca como problema.

El criterio de «hecho» de la Fase 7 es que una persona que no conoce Sift lo usa sin explicaciones. Ahora mismo no se cumple en el primer arranque.

Además, la descarga del modelo es la única conexión que Sift hace a internet (ver [PLAN.md](../../PLAN.md) y [ADR 0009](0009-busqueda-por-significado.md)). Por eso tiene que pedir permiso antes de hacerla.

## Decisiones

### 1. Dónde va la bienvenida

**Elegido:** una página de Ajustes en «modo bienvenida» la primera vez. No tiene navegación lateral, va centrada y con el título «Te damos la bienvenida a Sift» (forma neutra, decidida por Darío el 2026-11-17). Después de completarla queda como «Primeros pasos» en la navegación.

| Opción | A favor | En contra |
|---|---|---|
| **Página de Ajustes en modo bienvenida** | Reutiliza la ventana, los controles y el vidrio de Ajustes. No toca la apertura de la barra (< 100 ms). | Es una ventana más que abrir la primera vez. |
| Dentro de la barra | No añade ventana. | La barra es pequeña, se cierra al perder el foco y elegir carpetas abre diálogos del sistema. |
| Ventana propia | Aislada del resto de la app. | Otra ventana que mantener: material, barra de título, foco y tamaño propios. |

### 2. Cuándo sale

**Elegido:** solo en un equipo nuevo, es decir, cuando no se ha visto antes (`welcomeDone` no está marcado) y no hay carpetas ni Espacios.

- Un `settings.json` antiguo que no tiene el campo `welcomeDone` cuenta como visto.
- Unos datos que ya tienen carpetas o Espacios marcan la bienvenida como vista al arrancar.

Así no sale a quien ya usa Sift ni a los datos de desarrollo.

| Opción | A favor | En contra |
|---|---|---|
| **Solo en equipo nuevo** (sin `welcomeDone`, sin carpetas ni Espacios) | No molesta a quien ya usa la app. | Quien borre sus carpetas y Espacios a propósito no la vuelve a ver, aunque siempre puede abrirla desde «Primeros pasos». |
| Siempre que no haya carpetas | Más sencillo de explicar. | Vuelve a salir a quien quita sus carpetas, y a los datos de desarrollo. |

### 3. El modelo nunca se descarga sin un «sí»

**Elegido:** un ajuste `modelDownloadAllowed`, falso por defecto. Si el modelo ya está en disco, se usa igual. Sin permiso:

- el estado de la búsqueda por significado es `awaitingPermission` («esperando permiso»);
- Ajustes → General y la bienvenida muestran el botón «Descargar (120 MB)».

La descarga sigue con su hash fijado, como dice [ADR 0009](0009-busqueda-por-significado.md).

| Opción | A favor | En contra |
|---|---|---|
| **Permiso explícito, falso por defecto** | Ninguna conexión sin que el usuario lo pida. Cumple la regla de descargas de [CLAUDE.md](../../CLAUDE.md). | Quien actualice sin el modelo descargado no tiene búsqueda por significado hasta que lo pulse. |
| Descargar al arrancar (comportamiento anterior) | Búsqueda por significado lista sin pasos extra. | Descarga 120 MB sin preguntar. Es lo que señala la auditoría. |

### 4. Pasos de la bienvenida

1. **El atajo:** muestra `Alt+Espacio` (o el que tenga configurado) y cómo abrir la barra.
2. **Dónde buscar:** Documentos y Escritorio, sugeridas por defecto. La interfaz no envía rutas: solo elige entre esas dos, o «Elegir otra carpeta…».
3. **Búsqueda inteligente:** «Descargar» o «Ahora no».

Pie: «Crear tu primer Espacio» o «Empezar». «Empezar» enseña durante 1,5 s «Pulsa <atajo> para empezar» y abre la barra.

## Consecuencias

- Un usuario que actualice sin tener el modelo descargado deja de tener la búsqueda por significado hasta que pulse «Descargar». Es el comportamiento buscado: la descarga pasa a ser decisión suya.
- La bienvenida reutiliza componentes de Ajustes, así que no añade superficies nuevas que mantener.
- Los datos de desarrollo y los de quien ya usa Sift no ven la bienvenida.
