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
| Reiniciar (tras morir) | R                    | A o Start                |
| Salir     | Esc                          |                          |

La torreta y el cañón giran hacia donde apunta la mira a una velocidad limitada. El círculo
pequeño marca dónde iría el disparo ahora mismo; se pone cian cuando el cañón está cargado.

## Estructura

- `src/input.rs`: junta teclado, ratón y mando en un único `Controls`.
- `src/tank.rs`: movimiento del casco, giro de la torreta y elevación del cañón.
- `src/world.rs`: estado del juego (cajas, enemigos, proyectiles, restos, vida, cámara) y colisiones.
- `src/render.rs`: dibujo del mundo y el HUD.
- `src/models.rs`: carga los GLB del tanque y de la torreta enemiga y los dibuja con sus piezas animadas.
- `src/model_layout.rs`, `src/turret_layout.rs`, `src/props_layout.rs`: generados por el exportador; no se editan a mano.
- `src/math.rs`: ángulos, cajas y rayos.

## Enemigos

Torretas fijas (`art/torreta/model.py`) repartidas por el mapa. Cuando estás a menos de 65 m giran
hacia ti, anticipan tu movimiento y disparan con sus dos cañones alternos; no disparan si una caja les
tapa el tiro. Aguantan 4 impactos y reaparecen en otro sitio a los 8 segundos. Cada impacto que recibes
quita 12 de blindaje; a 0 el tanque queda destruido y puedes volver a empezar.

## Arte

- `art/tanque/model.py`: el tanque en 3dixel (cubos, pivotes y texturas). Casco, torreta, cañón y cada rueda son grupos separados.
- `art/torreta/model.py`: la torreta enemiga (base, cabeza y cañones dobles).
- `art/props/model.py`: escenario: caja militar (destructible), rocas y barrera de hormigón (cobertura fija; las barreras también cierran la arena).
- `art/terreno/ground.py`: pinta la textura del suelo (`assets/textures/ground.png`, 64x64 que se repite cada 6,4 m).
- `art/common/paint.py`: paletas y pintores de textura compartidos.
- `tools/3dixel/`: copia del núcleo de 3dixel (licencia MIT) para poder renderizar y exportar sin nada más.
- `tools/export_glb.py`: exporta las piezas de cada modelo a GLB y genera un archivo Rust con sus pivotes. Cada `model.py` dice qué exportar con `exports()` y `layout()`.

Después de cambiar el modelo (hace falta Python 3 con numpy y Pillow):

```sh
python3 tools/3dixel/core/render.py art/tanque --out /tmp --name tanque   # vistas previas
python3 tools/export_glb.py art/tanque/model.py assets/models/tank --layout src/model_layout.rs
python3 tools/export_glb.py art/torreta/model.py assets/models/turret --layout src/turret_layout.rs
python3 tools/export_glb.py art/props/model.py assets/models/props --layout src/props_layout.rs
python3 art/terreno/ground.py assets/textures/ground.png
```

En el juego las tiras cian y las toberas brillan sin depender de la luz (textura emisiva y el shader `assets/shaders/lit.*`), y las ruedas giran según lo que avanza el tanque.

## Pruebas

```sh
cargo test
cargo run -- --screenshot captura.png   # juega unos segundos con entrada fija y guarda una captura
cargo run -- --screenshot lado.png --yaw 100 --pitch -15 --frames 300   # otro ángulo y más tiempo de partida
```
