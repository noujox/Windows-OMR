# Port de Meeting Recorder a Windows (en desarrollo)

> Rama de trabajo: `port/windows-native`. Base: upstream `5d098eb`. No es todavía una versión Windows compilable.

## Alcance decidido

- Aplicación de escritorio independiente de Omarchy y de Hyprland.
- Mantener Rust, GTK4/libadwaita y el formato de las reuniones existentes.
- Micrófono: dispositivo predeterminado del sistema.
- Audio del computador: salida predeterminada mediante WASAPI Loopback.
- Motor STT: Whisper.cpp (Whisper-rs).
- Aceleración: automática con fallback efectivo a CPU si falla GPU.
- Minimizar a bandeja y arrancar con Windows: habilitados por defecto.
- Al arrancar con Windows nunca comenzar grabación automáticamente.
- Conservar la compatibilidad futura con Linux.

## Avances implementados

- [x] Rama independiente.
- [x] Eliminar plugin de barra, integración QML, scripts AUR/Omarchy, y oferta para instalar el widget.
- [x] Eliminar funciones hyprctl y gestión especial de posición/tamaño para Hyprland; dejar a GTK gestionar ventanas.
- [x] Separar paleta visual de archivos colors.toml de Omarchy.
- [x] Seleccionar agente IA por `MEETING_RECORDER_AGENT` o `agent = "codex"` de config.toml.
- [x] Añadir preferencias persistentes de dispositivos, Whisper, aceleración, tray y autostart (valores por defecto).
- [x] Directorios de preferencias/configuración y modelos preparados para usar rutas de usuario GLib en Windows.
- [x] Selección de aceleración Whisper auto/CPU/GPU; en auto, reintento CPU si falla inicialización GPU (sin pruebas de GPU Windows todavía).
- [x] Captura abstraída por roles lógicos (micrófono y audio del computador), conservando por ahora el backend Linux.

## Pendiente antes de compilar/ejecutar en Windows

- [ ] Backends nativos de captura WASAPI para micrófono y loopback; preservar la sincronización y silencio compensatorio.
- [ ] Sustituir el reproductor `pacat` por WASAPI o un reproductor multiplataforma.
- [ ] Sustituir el socket Unix de IPC por transporte Windows seguro (preferiblemente Named Pipes con ACL de usuario).
- [ ] Adaptar manejo Unix de procesos en player, webcam y agent, conservando límites de seguridad.
- [ ] Captura webcam mediante dispositivos Windows (Media Foundation o FFmpeg dshow), incluida vista previa.
- [ ] Validar política de aceleración GPU/CPU con dispositivos reales Windows y empaquetar los runtimes GPU pertinentes.
- [ ] Implementar bandeja, minimizar y restaurar; mantener grabación al ocultarse.
- [ ] Implementar autostart del usuario, con ventana oculta y sin grabar.
- [ ] Reemplazar `sh -c` en actions por backend Windows apropiado.
- [ ] Nuevo paquete Windows con GTK4/libadwaita, FFmpeg y runtime Whisper; pruebas reales x64.
- [ ] Construcción, test de integración audio con Meet/Zoom/Teams; distribución e instalador.
- [ ] Desacoplar el ID/nombre upstream solo después de decidir nombre comercial y estrategia de migración.

## Estructura de las dependencias actuales

| Elemento | Estado |
| --- | --- |
| `src/ui.rs` | Conservado; sin oferta de plugin |
| `src/theme.rs` | Paleta libadwaita independiente |
| `src/settings.rs` | Preferencias persistentes definidas |
| `src/agent.rs` | Agente configurable; sandbox/ejecución Unix aún pendiente Windows |
| `src/audio.rs` | `parec` Linux, reemplazo pendiente |
| `src/player.rs` | `pacat` Linux, reemplazo pendiente |
| `src/ipc.rs` | Socket Unix; reemplazo pendiente |
| `src/webcam.rs` | V4L2 Linux; reemplazo pendiente |
| `src/transcribe.rs` | Whisper mantenido; aceleración auto con reintento CPU implementada, pendiente validación de hardware |
| `src/models.rs` | Almacenamiento Windows preparado |
| `src/actions.rs` | Shell Unix todavía requiere backend Windows |

## Validación de regresión Linux (no valida Windows)

```sh
docker build -f Dockerfile.linux-check -t meeting-recorder-linux-check .
docker run --rm --cpus=3 --memory=8g -v "$PWD:/work" -w /work meeting-recorder-linux-check cargo check --locked --no-default-features
```

Estos comandos construyen y validan únicamente el backend Linux, sin instalar Rust/GTK en el servidor anfitrión. El chequeo `cargo check --locked --no-default-features` pasó correctamente con Rust 1.92.0; quedaron advertencias `dead_code` por ajustes Windows todavía no conectados a la interfaz.

## Notas de diseño

No reemplazar el IPC con un puerto TCP sin autenticación: el estado de la reunión puede contener nombres y rutas sensibles. Preferir pipes restringidos al usuario.

La configuración de inicio automático es una preferencia inicialmente habilitada, **no** una acción de sistema hasta implementar la integración Windows.

El puerto debe continuar con fases verificables y commits pequeños. No modificar `main`.
