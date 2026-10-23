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
- [x] Captura por roles lógicos con backend WASAPI Windows y PulseAudio Linux.
- [x] Silencios finales sincronizados incluso cuando loopback no entrega ningún paquete.
- [x] Captura y reproducción cross-compiladas con Rust para `x86_64-pc-windows-gnu` (sin probar aún en hardware).

## Pendiente antes de compilar/ejecutar en Windows

- [x] WASAPI nativo: captura de micrófono predeterminado y loopback de salida, con reconexión ante cambio de dispositivo.
- [x] Reproducción WASAPI nativa en Windows; en Linux se conserva `pacat`.
- [x] IPC Windows implementado con Named Pipes y DACL restringida (SYSTEM y propietario), manteniendo `start`, `stop`, `pause`, `watch` y `wait`; comprobado mediante compilación cruzada, pendiente prueba de integración.
- [ ] Terminar manejo nativo de procesos: FFmpeg y webcam tienen compatibilidad básica, pero faltan Windows Job Objects para finalizar subprocesos tras caídas. Agente IA se deshabilita de forma segura en Windows hasta disponer de un runner aislado.
- [x] Adaptación inicial de webcam Windows a FFmpeg DirectShow, con enumeración y previsualización/segmentos; pendiente prueba física y validación de nombres especiales de cámaras.
- [ ] Validar política de aceleración GPU/CPU con dispositivos reales Windows y empaquetar los runtimes GPU pertinentes.
- [x] Implementación inicial de bandeja Windows con acciones Abrir, Grabar, Pausar, Detener y Salir; ocultar ventana al minimizar, manteniendo el proceso vivo. Pendiente prueba en Windows real.
- [x] Inicio automático mediante HKCU Run en Windows con `--start-hidden`, sin iniciar grabaciones. Pendiente verificar ciclo de vida y desinstalación en Windows real.
- [x] Ejecutar acciones configuradas en Windows mediante PowerShell no interactivo, conservando variables de entorno `MEETING_*`.
- [ ] Empaquetar Windows incluyendo GTK4/libadwaita, FFmpeg y runtime ONNX/Whisper; pruebas reales x64. Hay script MSYS2 y CI definidos, pero el chequeo del ejecutable completo aún no ha sido ejecutado.
- [ ] Construcción, test de integración audio con Meet/Zoom/Teams; distribución e instalador.
- [ ] Desacoplar el ID/nombre upstream solo después de decidir nombre comercial y estrategia de migración.
- [ ] Confirmar integración de icono de bandeja, autostart y minimización en Windows real; verificar aplicaciones GTK4 empaquetadas.
- [ ] Implementar Windows Job Objects para matar procesos multimedia si la app termina inesperadamente.
- [ ] Implementar runner seguro de agentes Windows; hasta entonces, generación de capítulos por agente está deshabilitada deliberadamente.

## Estructura de las dependencias actuales

| Elemento | Estado |
| --- | --- |
| `src/ui.rs` | Conservado; sin oferta de plugin |
| `src/theme.rs` | Paleta libadwaita independiente |
| `src/settings.rs` | Preferencias persistentes definidas |
| `src/agent.rs`, `src/agent_windows.rs` | Agente con aislamiento Linux; Windows deshabilitado provisionalmente hasta runner seguro |
| `src/audio.rs`, `src/audio_windows.rs` | Captura PCM PulseAudio Linux / WASAPI Windows |
| `src/player.rs`, `src/player_windows.rs` | FFmpeg + `pacat` Linux / render WASAPI Windows |
| `src/ipc.rs`, `src/ipc_windows.rs` | Unix socket Linux / Named Pipes Windows con DACL restringida |
| `src/webcam.rs` | V4L2 Linux / FFmpeg DirectShow Windows, pendiente validar cámara real |
| `src/transcribe.rs` | Whisper mantenido; aceleración auto con reintento CPU implementada, pendiente validación de hardware |
| `src/models.rs` | Almacenamiento Windows preparado |
| `src/actions.rs` | `sh` Linux / PowerShell no interactivo Windows |

## Validación de regresión Linux (no valida Windows)

```sh
docker build -f Dockerfile.linux-check -t meeting-recorder-linux-check .
docker run --rm --cpus=3 --memory=8g -v "$PWD:/work" -w /work meeting-recorder-linux-check cargo check --locked --no-default-features
docker run --rm --cpus=3 --memory=8g -v "$PWD:/work" -w /work meeting-recorder-linux-check cargo test --locked --no-default-features
```

Estos comandos construyen y validan únicamente el backend Linux, sin instalar Rust/GTK en el servidor anfitrión. El chequeo `cargo check --locked --no-default-features` pasó correctamente con Rust 1.92.0, y `cargo test --locked --no-default-features` pasó **67 pruebas de 67**. Quedaron advertencias `dead_code` por ajustes Windows todavía no conectados a la interfaz.

## Validación de código WASAPI Windows

El proyecto auxiliar compila los archivos reales `audio.rs`, `audio_windows.rs` y `player_windows.rs` usando Rust para Windows sin requerir GTK4 en el contenedor Linux:

```sh
docker run --rm -v "$PWD:/work" -w /work/tools/windows-audio-check meeting-recorder-linux-check sh -c 'rustup target add x86_64-pc-windows-gnu && cargo check --locked --target x86_64-pc-windows-gnu'
```

**Resultado:** comprobación cruzada WASAPI, Named Pipes y dependencias básicas de bandeja correcta; pruebas Linux 67/67. Estas validaciones todavía no prueban el ejecutable GTK en Windows. Faltan pruebas de hardware real, la compilación Windows de GTK4 y las demás dependencias Unix.

## Compilación Windows (en curso en TuxTeam)

- El entorno requerido es **MSYS2 UCRT64**, con toolchain Rust `x86_64-pc-windows-gnu`; no utilizar Rust MSVC como host cuando se compila con el GCC de MSYS2.
- Se agregó `vendor/whisper-rs-sys` 0.15.0 con un parche mínimo: aplicar `/utf-8` únicamente si `target_env = "msvc"`. La dependencia original falla al pasar esa bandera al GCC de MinGW.
- Para Windows, `ort` utiliza la opción `load-dynamic`: ONNX Runtime se carga como DLL compatible en tiempo de ejecución. Debemos incluir la DLL en la distribución; Linux sigue con su configuración original.
- La fuente original de la rama está compartida por SMB en `Z:\\miniCosas\\omarchy-meeting-recorder`; compilar sobre el disco local de TuxTeam y copiar luego artefactos a la carpeta SMB.

## Automatización de compilación (pendiente de validación CI)

- `scripts/build-windows-msys2.sh`: script para compilar Windows GNU CPU con MSYS2 UCRT64.
- `.github/workflows/windows-ci.yml`: comprobación del ejecutable Windows con GTK4/libadwaita en un runner real; solo correrá si se publica la rama o abre un PR.
- `src/windows_integration.rs`: integración HKCU Run y bandeja. Aún no probada en Windows.

## Notas de diseño

No reemplazar el IPC con un puerto TCP sin autenticación: el estado de la reunión puede contener nombres y rutas sensibles. Preferir pipes restringidos al usuario.

En Windows, `sync_startup()` registra o elimina el inicio automático en HKCU de acuerdo con la preferencia persistente. Esta integración solo se ejecuta cuando arranca la aplicación GUI, no al comprobar comandos CLI. Debe probarse en una sesión Windows real.

El puerto debe continuar con fases verificables y commits pequeños. No modificar `main`.
