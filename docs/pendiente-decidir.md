# Pendiente de decidir

Lo que salió trabajando de noche (2026-11-17/18) y necesita tu visto bueno. Cada punto tiene contexto, opciones y mi recomendación. Los marcados **decidido de noche** ya están hechos así porque eran pequeños y reversibles: si no te convencen, se cambian en un momento.

## Vista previa (bloque 7)

### 1. Tab sobre un Espacio o un reciente — decidido de noche
- **Contexto:** la vista previa enseña el texto guardado de un archivo encontrado. Los Espacios y lo «Abierto recientemente» no tienen fragmento.
- **Opciones:** (a) Tab no hace nada con ellos; (b) para un Espacio, enseñar su lista de acciones; (c) para un reciente, enseñar el principio del archivo.
- **Hecho:** (a). Si con la vista previa abierta bajas con ↓ hasta un Espacio, el panel dice «Este resultado no tiene vista previa.» en vez de cerrarse.
- **Recomendación para más adelante:** (b) es bonito y barato; al backlog.

### 2. Tamaño del código en la vista previa — decidido de noche
- **Contexto:** el texto normal va a 14 px; el código, en fuente fija, a 14 px no cabe en el ancho de la barra sin romper líneas.
- **Hecho:** código a 12 px (`text-xs`, de la escala de Vidrio), con sus saltos de línea y la indentación.

## Comprobación en pantalla (tarea 2)

### 3. Ver la bienvenida en la versión de desarrollo — decidido de noche
- **Contexto:** la versión de desarrollo se rellena sola con datos de prueba la primera vez, y con datos la bienvenida no sale (solo sale en un equipo nuevo). Renombrar la carpeta de datos no basta.
- **Hecho:** una variable solo para desarrollo, `SIFT_DEV_NO_SEED=1`, que arranca sin datos de prueba. En la versión diaria no existe (el código de los datos de prueba solo se compila en desarrollo).
