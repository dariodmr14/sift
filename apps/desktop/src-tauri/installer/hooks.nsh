; Textos del instalador de Sift (FEEDBACK 2026-11-19, bloque 8). Tauri incluye este archivo
; antes de definir las páginas, así que estos valores sustituyen a los de NSIS por defecto.
; Las imágenes (lateral y cabecera) salen de scripts/instalador-imagenes.ps1.

; La cabecera, a la derecha, como el resto de instaladores de Windows.
!define MUI_HEADERIMAGE_RIGHT

; Bienvenida.
!define MUI_WELCOMEPAGE_TITLE "Te damos la bienvenida a Sift"
!define MUI_WELCOMEPAGE_TEXT "Sift busca en tus archivos por su nombre, por lo que dicen y por lo que tratan, y abre de una vez todo lo que usas para una tarea.$\r$\n$\r$\nPulsa Siguiente para continuar."

; Final.
!define MUI_FINISHPAGE_TITLE "Sift está listo"
!define MUI_FINISHPAGE_TEXT "Ábrelo cuando quieras con Alt+Espacio."
!define MUI_FINISHPAGE_RUN_TEXT "Abrir Sift"
