# ADR 0005 — Dirección visual «Vidrio»

- **Estado:** aceptada
- **Fecha:** 2026-11-09

## Contexto

Tras la Fase 1, Darío pidió una interfaz simple pero con más personalidad, que luzca en un vídeo, sin animaciones lentas y respetando «reducir movimiento» (ver [FEEDBACK.md](../../FEEDBACK.md)). Se propusieron tres direcciones con mockups interactivos:

| Dirección | Idea | Por qué no |
|---|---|---|
| **Vidrio** | Integrada en Windows: Acrylic, Segoe UI, el acento del usuario | — elegida |
| Ficha | Fichero de biblioteca: papel claro, IBM Plex, fichas numeradas | Opaca y clara: no aprovecha el Acrylic; exigía diseñar un tema oscuro aparte |
| Tamiz | Tipográfica: Bricolage Grotesque, letras coincidentes en ámbar | La más vistosa, pero con una identidad que compite con Windows en vez de encajar |

## Decisión

**Vidrio**, como sistema de diseño con tokens (ver [docs/design-system.md](../design-system.md)):

- Tema y acento **los decide Windows**: Rust lee `AppsUseLightTheme` y `AccentPalette` del registro, con el mismo criterio que Fluent ("Light 2" sobre oscuro, "Dark 1" sobre claro). El tinte del Acrylic cambia con el tema.
- Tokens semánticos en CSS (oscuro y claro) expuestos a Tailwind con `@theme inline`. Un test impide usar colores literales en los componentes.
- Selección como en las listas de Windows: fondo sutil y píldora de 3 px del acento.
- Movimiento: entrada de 120 ms (opacidad 0,5 → 1, escala 0,98 → 1) y salida de 80 ms. Solo opacidad y transformaciones, y ninguno con «reducir movimiento».

## Consecuencias

- **Coste cero en arranque:** la fuente es la del sistema y no hay fuentes que cargar.
- **La personalidad viene del acento del usuario.** En un vídeo, Sift se verá con el color del PC que lo grabe; para la demo conviene un acento con carácter.
- **La entrada empieza en opacidad 0,5, no en 0** (como en el mockup), para no retrasar el primer fotograma visible. Medido con la animación: dentro del objetivo de < 100 ms (cifras en el README).
- En Windows 10 la ventana de la barra queda rectangular. Redondearla sin Windows 11 obligaría a recortarla con una región, que dentaría el borde.
- Cuando cambias el acento en Windows con Sift abierto, la barra lo recoge al volver a mostrarse; la ventana de ajustes, al reabrirse.
