# AimChanger

App de escritorio para Windows que convierte la sensibilidad del mouse de un juego a otro
(p. ej. VALORANT → DOOM Eternal) para que la sensación sea lo más parecida posible.
La UI y los mensajes al usuario están en **español**.

## Stack

- **Tauri 2** (shell nativo, WebView2) — instalador pequeño, un solo `.exe`.
- **Rust** (`src-tauri/`) — núcleo de conversión y base de datos de juegos. Toda la lógica vive aquí.
- **React 19 + TypeScript + Vite** (`src/`) — solo UI; llama a Rust vía `invoke`.

## Comandos

```bash
npm install                         # dependencias del frontend
npm run tauri dev                   # app en modo desarrollo (hot reload)
npm run tauri build                 # instalador en src-tauri/target/release/bundle/
npm run build                       # typecheck + build del frontend
cd src-tauri && cargo test          # tests del núcleo de conversión
cd src-tauri && cargo test capture -- --ignored   # test de Raw Input (mueve el cursor unos px)
cd src-tauri && cargo clippy        # lint de Rust
```

Requisitos (ya instalados en esta máquina): Rust toolchain `stable-x86_64-pc-windows-msvc`,
VS 2022 Build Tools con C++, WebView2, Node 22.

## Estructura

```
src-tauri/
  data/games.json      Base de datos de juegos (incrustada con include_str!)
  src/games.rs         Structs Game/FovSpec + carga de games.json
  src/conversion.rs    Matemática de conversión y de calibración + tests
  src/capture.rs       Captura pasiva de Raw Input del mouse (solo Windows)
  src/lib.rs           Comandos Tauri + atajo global F8
src/
  types.ts             Espejo TS de los structs serializados de Rust
  App.tsx              Pestañas + carga de juegos
  Converter.tsx        Pestaña "Convertir"
  Calibrator.tsx       Pestaña "Prueba 360°"
  SensFields.tsx       Campos/resultado de sensibilidad según el layout del juego
```

Comandos Tauri: `list_games`, `convert`, `calibration_arm`, `capture_toggle`,
`capture_status`, `calibrate`. Evento: `capture-toggled` (al pulsar F8).

## Modelo de conversión

Cada juego gira `yaw × sens` grados por *count* del mouse. `yaw` es la constante del motor
(VALORANT 0.07, Source/Quake/DOOM Eternal 0.022, etc.).

- **Distancia 360°** (default): `sens_dst = sens_src × yaw_src / yaw_dst`. Independiente del FOV.
- **Monitor Distance Match p%**: además multiplica por
  `atan(p·tan(hfov_dst/2)) / atan(p·tan(hfov_src/2))`; con p=0 se usa `tan(hfov_dst/2)/tan(hfov_src/2)`.
- `cm/360 = 360 / (yaw × sens × dpi) × 2.54`.
- El FOV de cada juego se normaliza a FOV horizontal en el aspect ratio del usuario.
  `FovType::Horizontal` se mide en `baseAspect`; `scaling` indica cómo cambia en otros aspect
  ratios: `horPlus` (conserva el vertical: CS2, Apex, DOOM) o `fixedHorizontal` (VALORANT, OW2).

Ejemplo de referencia: VALORANT 0.4 @ 800 DPI = 40.82 cm/360 → DOOM Eternal 1.273.

## Ajustes de sensibilidad por juego (`layout`)

Los valores del menú (`SensValues { main, x?, y? }`) se combinan en una sens efectiva por eje
(`conversion::effective`); `yaw × efectiva` = grados por count.

| `layout.kind` | Juegos | Efectiva X / Y |
|---|---|---|
| `single` (default) | VALORANT, CS2, Apex, OW2 | `main` / `main` |
| `separateXY` | Fortnite | `main` / `y` |
| `scaledXY` | DOOM Eternal | `main·x/ref` / `main·y/ref` (ref = 40) |

- Se conserva la relación vertical/horizontal del origen; si el destino es `single` se avisa.
- `conversion::decompose` hace lo inverso. Para `scaledXY` busca la combinación base × eje
  más precisa dentro de la precisión del menú y, en empate, el eje más cercano a su default.
- DOOM Eternal: "Escalado de la sensibilidad del ratón" = base con yaw 0.022; horizontal y
  vertical son multiplicadores (40 = ×1). Fórmula deducida de la comunidad, `verified.layout = false`.

## Prueba de 360° (calibración)

`yaw = 360 × vueltas / (|counts_x| × sens)`. El usuario pulsa F8 en el juego, da N vueltas y
pulsa F8 otra vez; `capture.rs` suma los counts horizontales netos.

- Captura **pasiva**: ventana message-only + `RegisterRawInputDevices` con `RIDEV_INPUTSINK`,
  recibe `WM_INPUT` aunque el juego tenga el foco. **Nunca** inyectar input (`SendInput`) desde
  la app: los anti-cheat (Vanguard) lo pueden detectar. `SendInput` solo se usa en el test ignorado.
- Eventos con `hDevice` nulo (inyectados por software/macros) se excluyen de la medición.
- F8 solo se registra como atajo global mientras la prueba está armada (`calibration_arm`).
- Raw Input es por proceso y por usage: si algo más en el proceso registra el mouse
  (usage page 1 / usage 2), el último registro gana. tao lo registra al arrancar; el nuestro
  se registra después, al armar la prueba.

## Agregar un juego

1. Añadir una entrada en `src-tauri/data/games.json` (`id` en kebab-case, único).
2. `yaw` siempre en grados por count **por unidad que muestra el menú**; si el menú no usa un
   número simple, indicar `unit` (p. ej. Fortnite: `"%"`, yaw 0.005555 por cada 1 %).
3. Poner `verified.*` en `false` para cualquier dato que no se haya comprobado en el juego.
4. Verificar el `yaw` con la pestaña "Prueba 360°" (error < 1.5 % con 3+ vueltas).
   Documentar la fuente en `notes` si aplica.
5. `cargo test` valida el JSON (rangos coherentes, ids únicos).

## Convenciones

- La lógica de conversión va **solo en Rust**; el frontend no recalcula nada.
- Si cambian los structs serializados en Rust (`#[serde(rename_all = "camelCase")]`),
  actualizar `src/types.ts` en el mismo cambio.
- Cada fórmula nueva en `conversion.rs` lleva un test con un valor conocido.
- No escribir en archivos de configuración de juegos sin confirmación explícita del usuario
  y sin hacer backup primero (VALORANT guarda la sens en la nube; escribir local no sirve).

## Pendiente / ideas

- Verificar en cada juego los datos con `verified.* = false` (rangos de sens; FOV de DOOM,
  Apex y OW2).
- Más juegos (R6 Siege, Marvel Rivals, PUBG…), sensibilidad de ADS/zoom.
- Fortnite: leer/escribir `MouseX_Sensitivity` en GameUserSettings.ini (valor del menú ÷ 100).
- Leer la sens actual desde archivos de config locales cuando el juego lo permita.
- Guardar yaws calibrados por el usuario y perfiles.
- Tecla de la prueba configurable (F8 puede chocar con atajos del juego).
