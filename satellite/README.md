# Meeting Recorder Satellite — BETA (Windows / Chrome, Edge)

**Función exclusiva de este fork de Windows**, no incluida en el proyecto original de Omarchy. Consulta la [guía principal en inglés](../README.md) para conocer su funcionamiento, instalación y uso.

Estado: **BETA experimental**. La detección es heurística y puede fallar cuando cambian las interfaces de Meet, Teams o Zoom. **Viene desactivada por defecto** en la extensión y en Meeting Recorder.

## Arquitectura

- Content script solo lee indicadores visibles de una llamada activa de **Google Meet, Teams web y Zoom web**. No lee audio, vídeo, participantes ni chats.
- Service worker MV3 valida el dominio, administra la sesión temporal y envía eventos por **Native Messaging**.
- Host nativo `meeting-recorder-satellite-host.exe` se comunica por Named Pipe con **una instancia ya abierta** de Meeting Recorder. No ejecuta ni abre Meeting Recorder.
- Un comando `satellite-join` inicia la grabación solo si la app no está grabando, deteniendo ni transcribiendo otra reunión. `satellite-leave` solicita pausa, nunca `stop`. Los latidos (`beat`) no pueden iniciar una grabación que haya sido omitida por una actividad manual.
- Si se pierde contacto con la pestaña, el watchdog pausa de forma conservadora después de 75 segundos sin latidos. Transcurridos 10 segundos desde la pausa se trae Meeting Recorder al frente para que el usuario elija Stop o Resume.
- Si se reconecta **antes** de finalizar el plazo, se reanuda solo una pausa causada por el satélite. La pausa manual tiene prioridad.

## Instalación en Windows (sin Chrome Web Store)

1. Usar una versión de Meeting Recorder compilada con la integración satélite. Dejar abierta la aplicación, aunque esté en bandeja.
2. Extraer el paquete satélite en una carpeta permanente (no mover posteriormente).
3. Abrir `chrome://extensions` o `edge://extensions`. Activar *Developer mode* y **Load unpacked** seleccionando la carpeta `extension` del paquete.
4. Copiar el ID de 32 caracteres de la extensión recién cargada.
5. En PowerShell ejecutar desde la raíz extraída: `powershell -ExecutionPolicy Bypass -File .\windows\install.ps1 -ExtensionId ID_COPIADO -Browser Vivaldi` si utilizas Vivaldi. Para Chrome o Edge, elegir `-Browser Chrome`, `Edge` o `Both`; `All` registra los tres. El ID se obtiene desde `vivaldi://extensions` (o `chrome://extensions`).
6. Reiniciar el navegador o deshabilitar/habilitar la extensión. Activar el interruptor en el popup y **Automatic browser meetings** dentro de Meeting Recorder. Ambos deben estar activos.

El instalador crea solamente claves de usuario HKCU, nunca requiere permisos de administrador. Es imprescindible registrar el ID exacto de la extensión mediante `allowed_origins`. No se abren puertos de red.

## Limitaciones de esta primera versión

- Los detectores DOM son **heurísticos**, sensibles a cambios de idioma e interfaz. La extensión puede no detectar algunas reuniones. El detector de Meet admite rutas `/codigo` y `/meet/codigo`, pero requiere que esté visible el botón de salir de la llamada. El popup diferencia entre falta de detección y error de enlace nativo. El control manual de la app siempre sigue disponible.
- Este paquete **no funciona con la versión anterior** de Meeting Recorder: necesita el binario que incluya los nuevos comandos.
- Si Meeting Recorder no está abierto, la extensión no inicia el programa.
- Nunca finaliza grabaciones automáticamente.
- Una reconexión pasada la ventana de revisión exige reanudar manualmente.
- Probar primero con llamadas de prueba, respetando el consentimiento de los participantes.
- No se hicieron pruebas funcionales de captura de reuniones en el computador del usuario.

## Código y pruebas

`src/satellite.rs`: estado aislado y pruebas unitarias.
`src/bin/meeting-recorder-satellite-host.rs`: host stdio de Native Messaging.
`satellite/extension`: extensión Manifest V3 para Chrome y Edge.
`satellite/windows/install.ps1`: registro por usuario.

El popup indica cuando **el host nativo** pudo enviar el mensaje al canal local, pero ese estado no confirma que la aplicación haya iniciado a grabar. Si una llamada estaba activa antes de habilitar la automatización, puedes pulsar **Reenviar inicio de reunión** en el popup: es una acción manual que solicita iniciar la grabación de la llamada activa, sujeta a las mismas protecciones de la app.

Logs de la aplicación: `%LOCALAPPDATA%\omarchy-meeting-recorder\logs\app.log`. El binario actualizado registra el tipo de evento en `ipc.satellite`, si la función estaba habilitada en `satellite.received`, y la decisión (por ejemplo `Start` o `None`) en `satellite.decision`. No registra IDs ni contenido de reuniones.
