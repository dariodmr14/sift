# Estado de la sesión (2026-11-24, noche)

El repo está listo para publicar: versión **1.0.0**, la clave pública del actualizador puesta y los secretos de firma creados en GitHub. Lo que queda lo hace Darío, en orden: [publicacion/pasos.md](publicacion/pasos.md).

## Fusionado en `main`

| PR | Qué |
|---|---|
| [#62](https://github.com/dariodmr14/Sift/pull/62) | Limpieza antes de hacerlo público (`docs/publicacion/limpieza.md`, decisión 14) |
| [#63](https://github.com/dariodmr14/Sift/pull/63) | Workflow de Release con etiqueta `v*`, actualizaciones encendidas por defecto (decisión 15, ADR 0018), `docs/publicacion/pasos.md` y notas de la v1.0.0 |
| [#64](https://github.com/dariodmr14/Sift/pull/64) | Datos de demostración en inglés (`demo-en/`, `SIFT_DEV_DEMO=en`) |
| [#66](https://github.com/dariodmr14/Sift/pull/66) | Script de la introducción, capturas de la rc.4 y `cookie` / `url` por dos avisos de `cargo audit` |
| [#67](https://github.com/dariodmr14/Sift/pull/67), [#68](https://github.com/dariodmr14/Sift/pull/68) | Estado de las sesiones anteriores |
| [#69](https://github.com/dariodmr14/Sift/pull/69) | La app enseña 135 MB como tamaño del modelo también antes de que Rust dé la cifra, con un test que lo vigila |
| Versión 1.0.0 | `Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json` y `tauri.conf.json`; clave pública (`1AE49F71D00F5175`) en `tauri.updater.conf.json` |

## Espera la revisión de Darío

[#65](https://github.com/dariodmr14/Sift/pull/65) (`docs/readme-publico`): README en español e inglés, `LICENSE` (autor «Darío Morales Riega», en español e inglés) y `docs/desarrollo.md`, con los GIF de la rc.4. Las cifras son solo las de la auditoría 1.0. CI local en verde. **No se fusiona sin su visto bueno.**

## Vídeo (fuera del repo: `Documents\Sift - vídeo de presentación\`)

Rehecho el 2026-11-24 con el diseño de sonido de la rc.2 (sus muestras) sobre la cámara nueva (Video Shotcraft): un sonido por cada cosa que aparece o se mueve, uno que agrupa cada ráfaga, cada muestra colocada por su ataque o su pico y nivelada para oírse sobre la música. −14,1 LUFS, 0 ms de desfase. Cómo está hecho: `LEEME.md` de esa carpeta. **Lo tiene que escuchar Darío.**

## Comprobado al terminar

- No queda abierta ninguna Sift de desarrollo ni procesos de HyperFrames.
- La Sift instalada y sus datos no se han tocado. La clave privada no se ha leído: solo `sift.key.pub`.
