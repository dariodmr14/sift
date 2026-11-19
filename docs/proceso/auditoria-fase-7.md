# Auditoría antes de la Fase 7

- **Fecha:** 2026-11-16
- **Alcance:** toda la app tal como está en `main` tras la Fase 6.5. Se revisó la barra, los Ajustes, el editor de Espacios, los estados vacíos y los errores, además del diseño frente a Vidrio, la accesibilidad, el rendimiento, la seguridad, el código, los tests y la documentación.
- **Cómo:** con las skills de ECC que aplican (lista abajo) y los criterios de [unslop](https://github.com/heydamir/unslop), leídos de su repositorio sin instalarlo todavía. Instalarla es la primera tarea de la Fase 7.
- **Límite:** la barra se revisó en el código, no en pantalla, porque para eso hace falta tu permiso. Los Ajustes, sí: con las capturas de la sesión anterior. Las capturas de la barra quedan para el paso 2.

Esfuerzo e impacto: **bajo / medio / alto**. Lo que toca el sistema Vidrio va marcado con 🟡 **PROPUESTA**: lo decides tú.

---

## 1. Skills de ECC: cuáles aplican

### Aplican

| Área | Skills | Para qué en Sift |
|---|---|---|
| Diseño e interfaz | **unslop** (externa), `frontend-design-direction`, `design-system`, `make-interfaces-feel-better`, `taste` / `taste-application`, `click-path-audit`, `product-lens` | La dirección de diseño, el pulido, el recorrido de cada flujo y la coherencia con Vidrio |
| Vidrio | `liquid-glass-design` | Solo como contraste: es el vidrio de Apple. Vidrio usa Acrylic de Windows, así que no se adopta. |
| Movimiento | `motion-foundations`, `motion-patterns`, `motion-advanced` | Animaciones de la Fase 7: entrada y salida, listas, arrastre |
| Accesibilidad | `accessibility`, `frontend-a11y` | WCAG 2.2: foco, contraste, tamaño de los objetivos, lector de pantalla |
| Frontend | `react-patterns`, `react-performance`, `react-testing`, `typescript-patterns`, `vite-patterns`, `frontend-patterns` | Estructura de `App.tsx`, renderizado, tests de componentes |
| Rust | `rust-patterns`, `rust-testing`, `error-handling`, `latency-critical-systems`, `benchmark` / `benchmark-methodology`, `database-migrations`, `content-hash-cache-pattern` | Errores y pánicos, latencia de la búsqueda, migraciones de SQLite, caché por hash (ya se usa) |
| Búsqueda | `eval-harness` y el agente `rag-pipeline-reviewer` (por su criterio) | Calidad de `eval/`, duplicados y *reranker* (backlog) |
| Seguridad | `security-review`, `security-scan` (AgentShield) | IPC, CSP, comandos, descargas; la configuración de `.claude/` |
| Calidad y tests | `coding-standards`, `plankton-code-quality`, `production-audit`, `repo-scan`, `verification-loop`, `tdd-workflow`, `e2e-testing`, `windows-desktop-e2e`, `browser-qa`, `test-coverage` | Tamaño de los módulos, cobertura y pruebas de extremo a extremo |
| Documentación | `docs-governance`, `living-docs-governance`, `architecture-decision-records`, `codebase-onboarding` | Guía de Vidrio al día, README para gente de fuera, ADR |
| Git | `git-workflow`, `conventional-commit-review`, `github-ops` | Ya se sigue (CLAUDE.md) |
| Para el README, la web o el vídeo (Fase 8) | `brand-voice`, `brand-discovery`, `content-engine`, `article-writing`, `competitive-platform-analysis`, `seo`, `marketing-campaign`, `ui-demo`, `remotion-video-creation`, `video-editing`, `i18n-sync` | Posicionamiento frente a Raycast, PowerToys Run y Everything; vídeo de LinkedIn; una web futura; traducir al inglés si se publica |

### No aplican (una línea por familia)

- **Lenguajes y frameworks que Sift no usa:** Python, Django, FastAPI, Java, Spring, Quarkus, JPA, MyBatis, Kotlin, Android, KMP, Compose, Swift, SwiftUI, Flutter, Dart, Go, C++, C#, .NET, F#, Perl, PHP, Laravel, Rails, Vue, Nuxt, Angular, Next.js, NestJS, Bun, Prisma, Postgres, MySQL, Redis, ClickHouse, Docker, Kubernetes, Tinystruct.
- **Dominios ajenos:** salud (HIPAA, EMR, CDSS), finanzas y facturación, logística, aduanas, energía, compras, cadena de bloques (DeFi, EVM, x402), mercados de predicción, redes y homelab (Cisco, BGP, VLAN, WireGuard), bases científicas, inversores.
- **Infraestructura de agentes y operaciones:** bucles autónomos, `agentic-os`, `council`, `devfleet`, `hermes`, `nanoclaw`, orquestación de equipos, coste de tokens, instintos, `jira`, `epic-*`, correo, mensajería y redes sociales.
- **Multimedia no relacionada:** Manim, Blender, fal.ai, VideoDB, vídeos de marcas ajenas.

---

## 2. Lo que está bien (no tocar)

Para no perderlo al pulir:

- **Interfaz sin HTML crudo.** No hay `innerHTML`. Los iconos SVG van como `data:` en `<img>`/`mask`, donde los scripts no se ejecutan.
- **Comandos seguros.** Siempre programa y argumentos separados, sin shell. Las esperas solo miran direcciones de tu equipo, sin redirecciones ni proxy.
- **Descargas comprobadas.** El modelo y PDFium tienen su hash fijado.
- **Logs.** No guardan búsquedas. `npm audit` está limpio.
- **Movimiento.** Se respeta «reducir movimiento».
- **Idioma y carga.** `lang="es"` está en las dos ventanas. La librería de iconos (450 KB) solo se carga al abrir el selector.
- **Contraste de los tokens:** oscuro ~8:1 en el texto atenuado; claro 5,5:1 (atenuado), 5,7:1 (acento) y 5,1:1 (peligro).
- **Calidad del código.** 257 tests de Rust y 102 de la UI. `clippy` sin avisos y `unsafe` prohibido salvo donde se justifica.

---

## 3. Hallazgos

### 🔴 Arreglar ya (fallos y riesgos)

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| A1 | En release, `panic = "abort"`: un fallo inesperado en **cualquier** hilo cierra Sift entero, sin aviso. | Si un extractor falla con un archivo raro (un PDF dañado, por ejemplo), Sift se cierra. Al arrancar vuelve a leer ese mismo archivo y se cierra otra vez: un **bucle de cierres**. Hoy no ha pasado, pero no hay nada que lo impida. | `error-handling`, `rust-patterns`, `production-audit` | medio | alto |
| A2 | Si el hilo de indexado muere, nadie se entera. `sync()` y `smart_changed()` hacen `let _ = send(...)` y el estado se queda en lo último. | Arreglado A1, un fallo dejaría el indexado parado en silencio: los cambios en tus carpetas no se verían y Ajustes diría «Al día». | `error-handling` (fallos silenciosos) | medio | medio |
| A3 | La CSP es mínima: `default-src 'self'; style-src 'self' 'unsafe-inline'`. Le faltan `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'` e `img-src 'self' data:` explícito, y `connect-src ipc:`. | Es defensa en profundidad y, al revisarlo, **un posible fallo real**: Tauri solo aplica la CSP en la versión instalada (nunca en desarrollo), y sin `img-src data:` los iconos de la barra se bloquearían ahí. `'unsafe-inline'` en estilos hace falta por los `style={…}` de React; queda anotado como deuda. | `security-review` | bajo | alto |
| A4 | La aprobación de comandos llega como `approve: true` desde la UI (`run_space`, `test_action`), y Rust se la cree. | Si algún día entrara contenido no confiable en el WebView, podría aprobar y ejecutar comandos. **Corrección al hacerlo:** un token de un solo uso no protege, porque quien llame desde la UI lo recibe. La confirmación tiene que salir del WebView: un **diálogo de Windows** desde Rust. | `security-review` | medio | medio |

### 🟢 Fase 7 (pulido y diseño)

#### Dirección y diseño

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| F1 | No hay una **dirección de diseño** escrita: producto, carácter, mundo, momentos y firma, el paso 3 de unslop. | Sin ella, el «diseño final» se decide pantalla a pantalla. Son cinco líneas que te propongo y apruebas antes de tocar nada. | unslop, `frontend-design-direction` | bajo | alto |
| F2 | **Emoji como iconos** en los comandos de la barra (⚙️ Ajustes, ➕ Crear Espacio), junto a iconos de Windows y de Lucide. | Es uno de los signos de unslop: mezcla tres familias de iconos, y el emoji se ve distinto en cada Windows. Es mejor usar Lucide o Segoe Fluent Icons. | unslop, `design-system` | bajo | medio |
| F3 | 🟡 **PROPUESTA (toca Vidrio):** cabeceras de sección en **mayúsculas con espaciado** («ESPACIOS», «ATAJO PARA ABRIR SIFT», en 5 sitios). | unslop lo marca como signo de plantilla. Windows 11 y Fluent usan mayúscula inicial en seminegrita, que encajaría mejor con «se siente de Windows». Hoy es una regla de Vidrio. | unslop | bajo | medio |
| F4 | `docs/design-system.md` no recoge los componentes nuevos: `Select`, campo de ruta con sugerencias, miniaturas de zonas, paneles «en marcha» y «resumen», etiqueta de filtro. | La guía es lo que mantiene el sistema. Si se queda atrás, cada pantalla nueva reinventa. | `docs-governance`, `design-system` | bajo | medio |
| F5 | Los **números que cambian** no usan cifras tabulares: «Leyendo 32 de 120», los segundos de espera, los ms, los MB. | El texto baila al cambiar cada cifra. Es `tabular-nums` en esos sitios. | `make-interfaces-feel-better` | bajo | bajo |
| F6 | Títulos y ayudas sin `text-wrap: balance` / `pretty`. | Algunos párrafos de ayuda de Ajustes terminan con una palabra sola en la última línea. | `make-interfaces-feel-better` | bajo | bajo |
| F7 | **Pantalla de bienvenida:** la primera vez, la barra solo dice «Aún no tienes Espacios»; no te pide carpetas, y la búsqueda inteligente **descarga 120 MB sin preguntar**. | Es el criterio de la Fase 7: «alguien que no lo conoce lo usa sin explicaciones». Además, conviene pedir permiso para la única conexión de la app. | `product-lens`, `click-path-audit` | medio | alto |
| F8 | **Vista previa y «por qué sale este resultado»** (FEEDBACK y PLAN). En la barra no cabe. | Es lo que hace creíble la búsqueda por significado. Te propondré el formato antes de hacerlo: panel lateral, o una tecla (→ o Espacio). | `product-lens`, `frontend-design-direction` | alto | alto |

#### Accesibilidad

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| F9 | El **acento es el color de tu Windows** y su contraste no está garantizado. «Por significado», el aviso «!», las miniaturas de zonas y los botones principales (con texto negro fijo en oscuro) dependen de él. | Con un acento oscuro, por ejemplo azul marino o granate, esos textos se vuelven ilegibles para algunas personas. Habría que calcular al arrancar una variante del acento que cumpla 4,5:1 sobre la superficie. | `accessibility` | medio | alto |
| F10 | 🟡 **PROPUESTA (toca Vidrio):** los bordes de campos y controles (`--line`, 8 % de opacidad) se quedan muy por debajo de 3:1 frente al fondo. | WCAG 1.4.11 pide 3:1 para los límites de los controles. Hoy los campos se distinguen por el relleno, no por el borde. Sería subir el borde solo en los campos, no en todo Vidrio. | `accessibility` | bajo | medio |
| F11 | El **foco de los campos** solo cambia el color de un borde de 1 px. Los botones sí tienen anillo de 2 px. | Es un indicador débil (WCAG 2.4.11/2.4.13): con teclado cuesta ver dónde estás. Sería el mismo anillo que los botones. | `accessibility`, `frontend-a11y` | bajo | medio |
| F12 | Los **diálogos** (`Modal`) no atrapan el foco ni lo devuelven al cerrar; Esc sí funciona. | Con el tabulador se sale del diálogo a lo que hay detrás (WCAG 2.1.2). | `accessibility` | bajo | medio |
| F13 | La barra no **anuncia** cuántos resultados hay ni el filtro activo. | Con lector de pantalla no sabes si encontró algo hasta que te mueves. Basta una región `aria-live` oculta («12 resultados en tus PDF»). | `accessibility` | bajo | medio |
| F14 | El **asa de arrastre** (⠿) mide unos 16×20 px. | Por debajo del mínimo de 24×24 (WCAG 2.5.8). | `accessibility` | bajo | bajo |

#### Movimiento

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| F15 | **Arrastre de acciones y Espacios:** el color cambia de golpe al coger y soltar (FEEDBACK). | Es el único sitio con «saltos» visibles. Se arregla con una transición de fondo y sombra con los tokens de duración. | `motion-foundations`, `make-interfaces-feel-better` | bajo | medio |
| F16 | Los **resultados** aparecen de golpe al escribir, y los paneles (confirmar, en marcha, resumen) se cambian sin transición. | Una entrada breve, de opacidad y unos píxeles, sin tocar la apertura de la barra, haría que no parezca que salta. | `motion-patterns` | medio | medio |
| F17 | 🟡 **PROPUESTA:** un estado «pulsado» sutil en los botones (oscurecer, o `scale(0.98)`). | Fluent no escala al pulsar; oscurece. Lo dejo a tu elección para no salirnos de Windows. | `make-interfaces-feel-better` | bajo | bajo |

#### Código y tests (para poder pulir sin romper)

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| F18 | `App.tsx` tiene **851 líneas en un solo componente**: búsqueda, modos, Espacios en marcha, teclado y cuatro paneles. | La vista previa y las animaciones van justo ahí. Antes conviene separarlo en ganchos (`useFileSearch`, `useSpaceRun`) y en paneles en sus propios archivos. | `react-patterns`, `coding-standards` | medio | medio |
| F19 | **No hay tests de componentes.** Los 102 de la UI son de lógica pura; nada prueba el teclado de la barra, Retroceso con filtro o Esc al cancelar. | Las animaciones y el reordenado de la Fase 7 tocan justo eso. Se haría con React Testing Library. | `react-testing`, `tdd-workflow` | medio | medio |
| F20 | **No hay pruebas de extremo a extremo.** Lo que hice a mano en la Fase 6.5 (manejar la app real por el protocolo de DevTools del WebView2) se puede convertir en una prueba de humo: abrir la barra, buscar, abrir Ajustes. | Cazaría errores como el de la recarga de Ajustes, que solo se ven con la app de verdad. | `windows-desktop-e2e`, `browser-qa` | medio | medio |

### 🔵 Más adelante

| # | Qué es | Por qué importa | Skill | Esfuerzo | Impacto |
|---|---|---|---|---|---|
| M1 | `indexing.rs` tiene 1.148 líneas: cola, sincronización, búsqueda inteligente y estado juntos. | Se entiende, pero cuesta cambiarlo. Se podría dividir en módulos sin cambiar el comportamiento. | `rust-patterns` | medio | bajo |
| M2 | Sin **cobertura** medida (`cargo-llvm-cov`, la cobertura de vitest). | Sabríamos qué no está probado, en vez de suponerlo. | `rust-testing`, `test-coverage` | bajo | bajo |
| M3 | Sin `cargo audit` en `scripts/ci.ps1`. `npm audit` está limpio, pero no se ejecuta solo. | Sin Actions, nadie avisa de una dependencia con fallo conocido. | `security-review` | bajo | medio |
| M4 | **Búsqueda:** copias del mismo archivo seguidas (`clarin.txt` ×3), *reranker*, contexto del documento. Ya están en el backlog. | Siguiente salto de calidad, medido con `eval/`. | `eval-harness`, `rag-pipeline-reviewer` | medio–alto | medio |
| M5 | `eval/` con **consultas que fallen en tu uso real**: sigue pendiente de lo que vayas viendo. | Es lo que guía M4. | `eval-harness` | bajo | medio |
| M6 | **README para gente de fuera:** sin captura ni GIF, sin una sección de privacidad clara, y con «Lo aprendido» muy largo. Esto último iría a `docs/aprendido.md`. | Hoy es un diario del proyecto, y se nota. Para la Fase 8 tiene que vender en diez segundos. | `docs-governance`, `brand-voice` | bajo | medio |
| M7 | **Inglés:** todo está en español y sin capa de traducción. | Solo si se publica fuera. | `i18n-sync` | alto | medio |
| M8 | Web, vídeo de LinkedIn y comparación con Raycast, PowerToys Run y Everything. | Fase 8. | `seo`, `content-engine`, `ui-demo`, `competitive-platform-analysis` | medio | medio |
| M9 | `security-scan` (AgentShield) sobre la configuración de agentes del repositorio (`.claude/`). | Comprobaría que los permisos y los ganchos siguen siendo los mínimos. | `security-scan` | bajo | bajo |

---

## Revisión de `.claude/` (hecha con «Arreglar ya»)

- **Permisos del proyecto** (`.claude/settings.json`): se permiten `git add/commit/push` y `gh pr`/`gh run`, y se prohíbe forzar el push a `main`. Faltaba cerrar lo que CLAUDE.md prohíbe y nada impedía: **borrar `main` en GitHub** (`git push origin :main`, `git push --delete`), **forzar con `+rama`** a cualquier rama y `gh repo delete`. Ahora están prohibidos.
- **`settings.local.json`** (no se sube a git): solo activa el paquete ECC.
- **Ganchos de ECC:** 27 ganchos de Node que se ejecutan antes y después de cada herramienta y al empezar y acabar la sesión. Revisados sus scripts: ninguno sale a internet (el único que abre una conexión va a `localhost`, el lienzo de planes). Son de terceros y corren con tus permisos; si alguno estorba, se puede desactivar con `ecc:hookify-configure`.

## 4. Propuesta de orden para la Fase 7 (aprobada)

1. **Arreglar ya** (A1–A4), en un PR propio.
2. **Base para pulir:** separar `App.tsx` (F18), tests de componentes (F19) y la prueba de humo de extremo a extremo (F20).
3. **unslop:**
   - instalarla y citarla en el README con el texto que pide: *«unslop was created by Damir Sokolovsky ([@heydamir](https://x.com/heydamir)) … CC BY 4.0»*;
   - capturas reales de cada pantalla (con tu permiso);
   - auditoría, dirección de diseño (F1) para que la apruebes, y luego sus nueve pasadas respetando Vidrio y lo que decidas de F3, F10 y F17.
4. **Accesibilidad:** F9 a F14.
5. **Movimiento:** F15 a F17.
6. **Bienvenida** (F7).
7. **Vista previa y «por qué sale este resultado»** (F8), con una propuesta de formato antes.
8. **Guía de Vidrio** al día (F4), README, PLAN y lo aprendido.

Cada bloque con su CI local completa, su PR y su merge.
