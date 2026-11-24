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

; ---------------------------------------------------------------------------------------------
; WebView2 (el motor que dibuja la interfaz; ADR 0021).
;
; Tauri lo instala con su bootstrapper si no lo encuentra (`webviewInstallMode`:
; `embedBootstrapper` en tauri.conf.json), pero solo mira que el registro tenga una versión.
; Una entrada «0.0.0.0» o sin los archivos detrás se da por buena y Sift falla al abrir
; («Could not find the WebView2 Runtime»). Aquí se comprueba de verdad antes de copiar Sift:
; si falta, se vuelve a ejecutar el bootstrapper (sin permisos de administrador lo instala solo
; para el usuario) y, si sigue faltando, se avisa en español con el enlace oficial y no se
; instala nada.

!define SIFT_WEBVIEW2_GUID "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!define SIFT_WEBVIEW2_URL "https://go.microsoft.com/fwlink/p/?LinkId=2124703"

Var SiftWv2Version ; la versión instalada de verdad, o "" si falta
Var SiftWv2Pv
Var SiftWv2Dir
Var SiftWv2Warned

; Da por buena la versión de $SiftWv2Pv si no es «0.0.0.0» y msedgewebview2.exe existe.
!macro _SiftWebView2Check
  ${If} $SiftWv2Pv != ""
  ${AndIf} $SiftWv2Pv != "0.0.0.0"
    ${If} $SiftWv2Dir == ""
      ${If} ${FileExists} "$PROGRAMFILES32\Microsoft\EdgeWebView\Application\$SiftWv2Pv\msedgewebview2.exe"
      ${OrIf} ${FileExists} "$LOCALAPPDATA\Microsoft\EdgeWebView\Application\$SiftWv2Pv\msedgewebview2.exe"
        StrCpy $SiftWv2Version $SiftWv2Pv
      ${EndIf}
    ${ElseIf} ${FileExists} "$SiftWv2Dir\$SiftWv2Pv\msedgewebview2.exe"
      StrCpy $SiftWv2Version $SiftWv2Pv
    ${EndIf}
  ${EndIf}
!macroend

; Deja en $SiftWv2Version la versión de WebView2 instalada (para todo el equipo o solo para
; este usuario), o "" si no está.
!macro SiftWebView2Version
  StrCpy $SiftWv2Version ""
  ReadRegStr $SiftWv2Pv HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${SIFT_WEBVIEW2_GUID}" "pv"
  ReadRegStr $SiftWv2Dir HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\${SIFT_WEBVIEW2_GUID}" "location"
  !insertmacro _SiftWebView2Check
  ${If} $SiftWv2Version == ""
    ReadRegStr $SiftWv2Pv HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${SIFT_WEBVIEW2_GUID}" "pv"
    ReadRegStr $SiftWv2Dir HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\${SIFT_WEBVIEW2_GUID}" "location"
    !insertmacro _SiftWebView2Check
  ${EndIf}
!macroend

; El aviso, una sola vez aunque fallen las dos comprobaciones. En una instalación silenciosa
; no se enseña.
Function SiftWebView2Missing
  ${If} $SiftWv2Warned == 1
    Return
  ${EndIf}
  StrCpy $SiftWv2Warned 1
  MessageBox MB_YESNO|MB_ICONEXCLAMATION "Sift necesita WebView2, el componente gratuito de Microsoft que dibuja su ventana, y no se ha podido instalar (¿falta conexión a internet?).$\r$\n$\r$\nDescárgalo de la página oficial de Microsoft:$\r$\n${SIFT_WEBVIEW2_URL}$\r$\n$\r$\nInstálalo y vuelve a abrir este instalador.$\r$\n$\r$\n¿Abrir la página ahora?" /SD IDNO IDYES sift_wv2_open
  Return
  sift_wv2_open:
  ExecShell "open" "${SIFT_WEBVIEW2_URL}"
FunctionEnd

; Si la instalación de Tauri (sección WebView2) falla, para con su mensaje en la ventana:
; se añade el aviso con el enlace.
Function .onInstFailed
  !insertmacro SiftWebView2Version
  ${If} $SiftWv2Version == ""
    Call SiftWebView2Missing
  ${EndIf}
FunctionEnd

!macro NSIS_HOOK_PREINSTALL
  ; Al actualizarse desde Sift (/UPDATE), WebView2 ya estaba: Sift se estaba ejecutando.
  ${If} $UpdateMode <> 1
    !insertmacro SiftWebView2Version
    ${If} $SiftWv2Version == ""
      !if "${WEBVIEW2BOOTSTRAPPERPATH}" == ""
        !error "Falta el bootstrapper de WebView2: tauri.conf.json tiene que usar webviewInstallMode embedBootstrapper (ADR 0021)."
      !endif
      Push $0
      DetailPrint "Instalando WebView2…"
      File "/oname=$TEMP\SiftWebView2Setup.exe" "${WEBVIEW2BOOTSTRAPPERPATH}"
      ExecWait '"$TEMP\SiftWebView2Setup.exe" /silent /install' $0
      Delete "$TEMP\SiftWebView2Setup.exe"
      DetailPrint "El instalador de WebView2 acabó con el código $0."
      Pop $0
      !insertmacro SiftWebView2Version
      ${If} $SiftWv2Version == ""
        Call SiftWebView2Missing
        Abort "Falta WebView2: no se ha instalado Sift."
      ${EndIf}
    ${EndIf}
    DetailPrint "WebView2 $SiftWv2Version"
  ${EndIf}
!macroend
