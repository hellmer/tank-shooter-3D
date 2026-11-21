# Tank Shooter 3D

Shooter en tercera persona de un tanque, en Rust con [raylib](https://github.com/raylib-rs/raylib-rs).
Ahora mismo es un prototipo: el tanque está hecho con cajas hasta que esté listo el modelo de Blockbench.

## Cómo ejecutarlo

Hace falta Rust (stable), CMake y un compilador de C. En Debian/Ubuntu, además:

```sh
sudo apt install cmake clang libx11-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev libgl1-mesa-dev
```

Después:

```sh
cargo run --release
```

## Controles

| Acción    | Teclado y ratón              | Mando                    |
|-----------|------------------------------|--------------------------|
| Mover     | WASD o flechas               | Stick izquierdo          |
| Apuntar   | Ratón                        | Stick derecho            |
| Disparar  | Clic izquierdo o espacio     | RT (o RB)                |
| Salir     | Esc                          |                          |

La torreta y el cañón giran hacia donde apunta la mira a una velocidad limitada. El círculo
pequeño marca dónde iría el disparo ahora mismo; se pone cian cuando el cañón está cargado.

## Estructura

- `src/input.rs`: junta teclado, ratón y mando en un único `Controls`.
- `src/tank.rs`: movimiento del casco, giro de la torreta y elevación del cañón.
- `src/world.rs`: estado del juego (blancos, proyectiles, explosiones, cámara) y colisiones.
- `src/render.rs`: dibujo del mundo, el tanque provisional y el HUD.
- `src/math.rs`: ángulos, cajas y rayos.

## Pruebas

```sh
cargo test
cargo run -- --screenshot captura.png   # juega unos segundos con entrada fija y guarda una captura
```
