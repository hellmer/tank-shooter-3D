# Tank Shooter 3D

Shooter en tercera persona de un tanque, en Rust con [raylib](https://github.com/raylib-rs/raylib-rs).
Ahora mismo es un prototipo. El tanque está modelado con [3dixel](https://github.com/mankidev/3dixel) (modelos al estilo Blockbench, con texturas pintadas píxel a píxel).

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
- `src/render.rs`: dibujo del mundo y el HUD.
- `src/tank_model.rs`: carga los GLB del tanque y lo dibuja (torreta, cañón y ruedas animados).
- `src/model_layout.rs`: generado por el exportador; no se edita a mano.
- `src/math.rs`: ángulos, cajas y rayos.

## Arte: el tanque

- `art/tanque/model.py`: el modelo en 3dixel (cubos, pivotes y texturas). Casco, torreta, cañón y cada rueda son grupos separados.
- `tools/3dixel/`: copia del núcleo de 3dixel (licencia MIT) para poder renderizar y exportar sin nada más.
- `tools/export_glb.py`: exporta cada grupo a GLB en `assets/models/tank/` y genera `src/model_layout.rs` con los pivotes y las ruedas.

Después de cambiar el modelo (hace falta Python 3 con numpy y Pillow):

```sh
python3 tools/3dixel/core/render.py art/tanque --out /tmp --name tanque   # vistas previas
python3 tools/export_glb.py art/tanque/model.py assets/models/tank --layout src/model_layout.rs
```

En el juego las tiras cian y las toberas brillan sin depender de la luz (textura emisiva y el shader `assets/shaders/lit.*`), y las ruedas giran según lo que avanza el tanque.

## Pruebas

```sh
cargo test
cargo run -- --screenshot captura.png   # juega unos segundos con entrada fija y guarda una captura
cargo run -- --screenshot lado.png --yaw 100 --pitch -15   # lo mismo con otro ángulo de cámara
```
