# Datos de demostración de Sift

Archivos **inventados** para las capturas, el vídeo y las pruebas de la versión de desarrollo: la tienda online de una clienta, apuntes de bases de datos y de redes, los gastos del piso y un viaje. No son de nadie.

- Con `SIFT_DEV_DEMO=1`, la versión de desarrollo copia esta carpeta a `C:\Users\Public\Documents\Sift (demostración)`, la busca y crea unos Espacios de ejemplo. Así no sale ninguna ruta personal en pantalla. Ver `demo_folder` en `apps/desktop/src-tauri/src/dev.rs`.
- Los dos PDF se generan con `python scripts/demo-pdf.py demo`.
- La misma carpeta en inglés está en `demo-en/` (`SIFT_DEV_DEMO=en`), para el vídeo y las capturas en inglés.
