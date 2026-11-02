# AimChanger

App de escritorio para Windows que convierte tu sensibilidad del mouse de un juego a otro,
para que al cambiar de juego el apuntado se sienta igual.

> Ejemplo: VALORANT 0.4 a 800 DPI (40.82 cm/360°) → DOOM Eternal **1.273**.

## Funciones

- **Convertir**: elige juego de origen y destino, escribe tu sensibilidad y tu DPI, y obtén
  el valor exacto para el menú del juego destino, junto con tus cm/360°.
- **Dos métodos de conversión**:
  - **Distancia 360°** (recomendado): una vuelta completa necesita el mismo recorrido del mouse en ambos juegos.
  - **Monitor Distance Match**: iguala el movimiento a un punto concreto de la pantalla (0 %–100 %),
    teniendo en cuenta el FOV de cada juego.
- **Sensibilidad por eje**: respeta los juegos con X/Y separadas (Fortnite) o con multiplicadores
  (DOOM Eternal) y conserva tu relación vertical/horizontal.
- **Prueba 360°**: mide con tu propio mouse cuánto gira un juego para comprobar o calibrar sus datos.

## Juegos soportados

| Juego | Notas |
|---|---|
| VALORANT | FOV fijo de 103°. |
| Counter-Strike 2 | FOV fijo (90° en 4:3). |
| Apex Legends | FOV del menú medido en 4:3. |
| Overwatch 2 | |
| Fortnite | Sensibilidad X/Y separadas, en %. |
| DOOM Eternal | Escalado base + multiplicadores horizontal/vertical. |

El giro por count del mouse (`yaw`) está comprobado en todos los juegos. Otros datos todavía no:
los rangos del menú en todos los juegos, el FOV de DOOM Eternal, Apex, Overwatch 2 y Fortnite,
y la fórmula de los multiplicadores de DOOM Eternal.

## Cómo usar la Prueba 360°

1. Abre la pestaña **Prueba 360°**, elige el juego, tu sensibilidad actual, tu DPI y cuántas vueltas darás.
2. Pulsa **Activar prueba** y entra al juego (campo de práctica, sin enemigos).
3. Apunta a un punto fijo, pulsa **F8** (pitido agudo) y gira despacio hacia un solo lado las vueltas elegidas.
4. Cuando la mira vuelva exactamente al punto, pulsa **F8** otra vez (pitido grave).
   La app calcula el giro real del juego y lo compara con el esperado. Con 3 o más vueltas el error baja.

**¿Es seguro con anti-cheat?** La app solo **lee** el movimiento del mouse (Raw Input) de forma pasiva;
nunca mueve el mouse ni envía teclas al juego, ni lee o modifica su memoria. F8 solo queda
registrado como atajo mientras la prueba está armada.

## Instalación

Descarga el instalador desde la sección *Releases* (cuando esté disponible) o compílalo tú mismo.

Requisitos: Windows 10/11 con WebView2 (ya viene incluido en Windows 11).

## Desarrollo

Hecho con **Tauri 2** + **Rust** (toda la lógica de conversión) y **React 19 + TypeScript + Vite** (la interfaz).

Requisitos:

- [Rust](https://rustup.rs/) (toolchain `stable-x86_64-pc-windows-msvc`)
- Visual Studio 2022 Build Tools con la carga de trabajo de C++
- Node.js 22

```bash
npm install              # dependencias del frontend
npm run tauri dev        # app en modo desarrollo
npm run tauri build      # instalador en src-tauri/target/release/bundle/

cd src-tauri
cargo test               # tests de la matemática de conversión y de games.json
cargo clippy             # lint
```

### Estructura

```
src-tauri/
  data/games.json      Base de datos de juegos
  src/conversion.rs    Fórmulas de conversión y calibración
  src/capture.rs       Captura pasiva de Raw Input (Prueba 360°)
  src/lib.rs           Comandos Tauri y atajo F8
src/                   Interfaz React
```

### Agregar un juego

1. Añade una entrada en [`src-tauri/data/games.json`](src-tauri/data/games.json) con un `id` único en kebab-case.
2. `yaw` = grados que gira el juego por cada *count* del mouse, por unidad del menú.
3. Marca como `false` en `verified` cualquier dato que no hayas comprobado en el juego.
4. Verifica el `yaw` con la Prueba 360° (error < 1.5 % con 3+ vueltas).
5. Ejecuta `cargo test` para validar el JSON.

Más detalles técnicos (modelo de conversión, FOV, layouts de sensibilidad) en [`CLAUDE.md`](CLAUDE.md).

## Hoja de ruta

- Más juegos (Rainbow Six Siege, Marvel Rivals, PUBG…) y sensibilidad de ADS/zoom.
- Guardar perfiles y yaws calibrados.
- Leer/escribir la sensibilidad desde archivos de configuración (con backup y confirmación).
- Tecla de la Prueba 360° configurable.
