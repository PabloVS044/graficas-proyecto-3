# graficas-proyecto-3

Sistema solar pequeño que se explora desde una nave, dibujado con un renderer por
software escrito en Rust. Proyecto final de Gráficas por Computadora 2026.

`raylib` se usa solo para abrir la ventana, leer el teclado, mostrar el framebuffer y
escribir el texto del HUD. Las matrices de modelo, vista y proyección, la
rasterización, la prueba de profundidad y los shaders están escritos en este
repositorio.

**Estado:** base del renderer y vuelo de la nave en tercera persona. Escena de prueba
con un sol y un planeta que orbita y rota.

## Cómo se ejecuta

Requiere Rust. `raylib` se compila desde su código fuente, así que también necesita
CMake, un compilador de C y las bibliotecas de desarrollo de OpenGL y del sistema de
ventanas.

```bash
cargo run --release                          # ventana
cargo run --release -- --benchmark           # ms por cuadro en poses fijas de la nave, sin ventana
cargo run --release -- --screenshot captura  # captura-inicio.png, captura-superficie.png, ...
cargo test --release                         # pruebas
```

Se ejecuta desde la raíz del repositorio: los modelos se cargan de `assets/`.

### Controles

| Tecla | Acción |
|---|---|
| `W` / `S` | Acelerar / frenar |
| Flechas ↓ / ↑ | Subir / bajar la punta, como la palanca de un avión (halar sube, empujar baja) |
| Flechas ← / → | Girar a la izquierda / derecha |
| `I` | Invertir las flechas arriba y abajo (arriba sube) |
| `A` / `D` | Alabear (girar sobre su eje) |
| `Shift` | Turbo |
| `C` | Inspeccionar: orbitar alrededor de la nave con flechas y rueda; otra vez `C` para volver |
| `R` | Volver la nave al inicio |
| Espacio | Pausar la simulación |

La aceleración y los giros son progresivos. La física avanza en pasos fijos de 1/120 s,
así que la nave responde igual sin importar cuántos cuadros por segundo dibuje el equipo.
No se puede atravesar el sol ni el planeta.

## Estructura

```
src/
  main.rs            ventana, bucle, --benchmark y --screenshot
  config.rs          resolución, campo de visión, planos, semilla, rutas
  math/              Mat4 y Quat propios
  mesh/              Vertex, Mesh, esfera envolvente, cargador OBJ
  render/            cámara, contexto del cuadro, pipeline, rasterizador,
                     framebuffer, fondo, contadores
  scene/             SceneObject, Collider y la escena de prueba
  ship/              física de la nave, cámara en tercera persona y controles
  simulation.rs      avance del mundo en pasos fijos
  shaders/           Material, shaders y funciones de ruido
assets/
  nave_graficas.obj  la nave (Blender, 39 168 triángulos)
  sphere.obj         esfera provisional, la reemplaza la icosfera generada
```

## Convenciones

Para que las tres partes del proyecto encajen:

- **Sistema de coordenadas:** mano derecha, `y` arriba, la cámara mira hacia `-z`.
- **Matrices:** vectores columna. `A * B` aplica primero `B`. Una matriz de modelo se
  lee de derecha a izquierda: `traslación * rotación * escala`.
- **Escala uniforme** en las matrices de modelo: las normales se transforman con la
  misma matriz y solo se normalizan.
- **Triángulos** antihorarios vistos desde afuera. El rasterizador descarta los que dan
  la espalda.
- **Recorte:** los triángulos que cruzan el plano cercano se cortan antes de proyectar,
  así la cámara puede pegarse a una superficie sin que aparezcan agujeros.
- **Profundidad:** el búfer guarda `1/w`. Más grande es más cerca y 0 es el infinito.
- **Mallas** sin índices: cada tres vértices seguidos son un triángulo.
- **Tiempo:** todo lo animado depende de `Simulation::time` (segundos de simulación,
  llega a los shaders como `FrameContext::time`), nunca del número de cuadro.
- **Nave:** la punta del modelo mira hacia `+z` y el techo hacia `+y`. Su orientación
  es un cuaternión.
- **Luz:** el sol es una luz puntual en `FrameContext::light_position`. Cada shader
  calcula su dirección con `shaders::light_direction`.
- **Unidades:** el sol mide 5 de radio y los planetas están a decenas de unidades.

### Cómo agregar algo

- **Un shader:** variante nueva en `shaders::Material` con sus parámetros, archivo
  nuevo en `src/shaders/` con su función, y una rama en `shaders::shade`. Los patrones
  se calculan sobre `fragment.object_position` para que se muevan con el objeto.
- **Un objeto:** construir un `SceneObject` con su malla (`Arc<Mesh>`, compartida entre
  copias), su matriz de modelo y su material, y agregarlo a la lista del cuadro.
- **Un modelo de Blender:** exportar OBJ con triángulos o polígonos y normales, ponerlo
  en `assets/` y cargarlo con `Obj::load(ruta)?.to_mesh(Normals::Authored)`.
  `Normals::Smooth` promedia normales para superficies curvas.

## Rendimiento de referencia

Ryzen 7 8845HS, 800×450, `--benchmark` (varía según el estado del equipo):

| Pose de la nave | ms por cuadro |
|---|---:|
| Inicio | 4.9 |
| Junto a la superficie del planeta | 8.5 |
| Junto al sol | 6.3 |
