# ADR 0016 — Borde inferior de los campos y sin animación al escribir

- **Estado:** aceptada
- **Fecha:** 2026-11-17

## Contexto

La auditoría de la Fase 7 ([auditoria-fase-7.md](../auditoria-fase-7.md)) marca dos problemas:

- **F10:** los bordes de los controles quedaban por debajo de 3:1 frente al fondo. WCAG 1.4.11 pide 3:1 para los bordes que hacen falta para reconocer un control, y un campo sin borde visible se confunde con el texto de alrededor.
- **F16:** los resultados aparecían de golpe.

## Decisiones

Aprobado por Darío el 2026-11-17.

### 1. Los campos llevan trazo inferior de 3:1

**Elegido:** los campos (`.sift-field`) tienen un borde suave (`--line`) y un trazo inferior de `--line-strong`, que llega a 3:1 frente al fondo, como los TextBox de Fluent. Solo afecta a los campos, no a todo Vidrio.

| Opción | A favor | En contra |
|---|---|---|
| **Trazo inferior de 3:1 solo en los campos** | Cumple 1.4.11 sin cambiar el resto de Vidrio. Es el criterio de Fluent. | Un token más (`--line-strong`) que mantener. |
| Subir `--line` en todo Vidrio | Un solo token para todo. | Más ruido visual en tarjetas y separadores. |

### 2. Los paneles de la barra cambian con transición; escribir no anima la lista

**Elegido:** al pasar de vacía a resultados, a «sin resultados» o a los paneles de confirmar, en marcha y resumen, el contenido entra con `.sift-swap` (opacidad y 4 px, `--dur-enter`). Al escribir, la lista no se anima: se lee al instante. La apertura de la barra no cambia, así que sigue por debajo de los 100 ms.

| Opción | A favor | En contra |
|---|---|---|
| **Transición en los cambios de panel; sin animación al escribir** | La lista se lee al instante y no hay parpadeo al teclear. No afecta a la apertura. | Al escribir, los resultados siguen cambiando de golpe, a propósito. |
| Animar cada cambio de resultados al escribir | Más vivo. | Parpadeo en cada tecla y sensación de lentitud. |

## Consecuencias

- Las dos reglas quedan en [docs/design-system.md](../design-system.md): los campos, en *Accesibilidad*, y el cambio de panel, en *Movimiento*.
- Los campos de Ajustes tienen trazo inferior de 3:1 (la caja de la barra es otra cosa: va sin borde, sobre el vidrio). Cualquier campo nuevo usa `.sift-field` para heredarlo.
- Los resultados siguen cambiando de golpe al escribir. Es la decisión, no un descuido: solo los cambios de panel tienen transición.
