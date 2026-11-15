# graficas-proyecto-3

Sistema solar pequeño que se explora desde una nave, dibujado con un renderer por
software escrito en Rust. Proyecto final de Gráficas por Computadora 2026.

`raylib` se usa solo para abrir la ventana, leer el teclado, mostrar el framebuffer y
escribir el texto del HUD. Las matrices de modelo, vista y proyección, la
rasterización, la prueba de profundidad y los shaders están escritos en este
repositorio.

**Estado:** base del renderer. Escena de prueba con un sol, un planeta que orbita y
rota, y la nave.

## Cómo se ejecuta

Requiere Rust. `raylib` se compila desde su código fuente, así que también necesita
CMake, un compilador de C y las bibliotecas de desarrollo de OpenGL y del sistema de
ventanas.

```bash
cargo run --release                          # ventana
cargo run --release -- --benchmark           # ms por cuadro desde varias distancias, sin ventana
cargo run --release -- --screenshot cuadro.png
cargo test --release                         # pruebas
```

Se ejecuta desde la raíz del repositorio: los modelos se cargan de `assets/`.

### Controles (provisionales)

| Tecla | Acción |
|---|---|
| Flechas | Orbitar la cámara |
| Rueda del mouse | Acercar o alejar |
| Espacio | Pausar la simulación |
| `R` | Reiniciar la cámara |

## Estructura

```
src/
  main.rs            ventana, bucle, --benchmark y --screenshot
  config.rs          resolución, campo de visión, planos, semilla, rutas
  math/              Mat4 y Quat propios
  mesh/              Vertex, Mesh, esfera envolvente, cargador OBJ
  render/            cámara, contexto del cuadro, pipeline, rasterizador,
                     framebuffer, fondo, contadores
  scene/             SceneObject y la escena de prueba
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
- **Tiempo:** todo lo animado depende de `FrameContext::time` (segundos de
  simulación), nunca del número de cuadro.
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

Ryzen 7 8845HS, 800×450, `--benchmark`:

| Vista | ms por cuadro |
|---|---:|
| Lejos | 3.8 |
| Media | 4.1 |
| Cerca del sol | 6.7 |
