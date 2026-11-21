# graficas-proyecto-3

Sistema solar pequeño que se explora desde una nave, dibujado con un renderer por
software escrito en Rust. Proyecto final de Gráficas por Computadora 2026.

`raylib` se usa solo para abrir la ventana, leer el teclado, mostrar el framebuffer,
escribir el texto del HUD y reproducir el audio. Las matrices de modelo, vista y
proyección, la rasterización, la prueba de profundidad, los shaders y la síntesis del
sonido están escritos en este repositorio.

**Estado:** renderer, vuelo de la nave en tercera persona y sistema solar con cinco
planetas y una luna. El sol y cada planeta tienen su propio shader, y todos reaccionan
cuando la nave se acerca. Tres planetas tienen objetos sobre su superficie, repartidos
con una semilla. El sonido se sintetiza en código.

## Video del recorrido

<p align="center">
  <a href="https://youtu.be/klRDVPa6hzc">
    <img src="https://img.youtube.com/vi/klRDVPa6hzc/maxresdefault.jpg" alt="Video del recorrido" width="720">
  </a>
</p>

<p align="center"><a href="https://youtu.be/klRDVPa6hzc">Ver el video en YouTube</a></p>

## Integrantes

| Integrante | Contribución |
|---|---|
| Pablo Vásquez (`PabloVS044`) | Base del renderer: matrices y cuaterniones, mallas y cargador OBJ, pipeline, recorte contra el plano cercano y profundidad. Vuelo de la nave: física, colisión, cámara en tercera persona, controles y simulación en pasos fijos. Shader de la nave, sistema solar con cinco planetas y shader de Jungla. Modelo de la nave Raptor y menú de selección de naves. |
| Jonathan Tubac (`JonathanTubac`) | Shaders del sol, Ambar, Kryos, Ignis, Eco y la luna, y su reacción a la nave (`proximity`). Icosfera, niveles de detalle y descarte fuera de la vista. Gravedad, modo orbital y partículas del turbo. Sonido sintetizado y HUD. Modelo de la nave Comet. |
| Ivana Figueroa (`IvanaFD`) | Objetos de superficie: árboles, chozas, rocas de basalto y cristales de hielo generados en código, su reparto con semilla, su orientación sobre la esfera, su aparición al acercarse y sus shaders. Capturas a ras del suelo. Modelo de la nave Stinger. |

## Características que solicitamos evaluar

| Categoría | Característica | Pts | Dónde verla |
|---|---|---:|---|
| Planetas | 5 planetas distinguibles que orbitan, rotan y se ven de cerca: Jungla, Ambar, Kryos, Ignis y Eco | 50 | Volar hacia cada uno; `src/scene/planets.rs` |
| Superficie | 4 tipos de modelo propio: árbol y choza (Jungla), roca de basalto (Ignis), cristal de hielo (Kryos) | 20 | Bajar a la superficie; `src/mesh/shapes.rs` |
| Superficie | Distribución procedural reproducible con ruido y semilla | 10 | `N` cambia la semilla; `scene::surface::place` |
| Superficie | Posición y orientación sobre la superficie curva mientras el planeta rota | 5 | `scene::surface::surface_frame` |
| Renderizado | Shaders propios distintos: sol, Jungla, Ambar, Kryos, Ignis, Eco, luna, nave y objetos de superficie | 24 | `src/shaders/` |
| Renderizado | Shaders animados: plasma del sol, nubes de Jungla, lava de Ignis, tormenta de Ambar, auroras | 8 | Todos dependen de `FrameContext::time` |
| Renderizado | Generación y subdivisión de geometría antes de rasterizar (icosfera) | 10 | `src/mesh/icosphere.rs` |
| Navegación | Modo orbital para rodear un planeta sin cortar el vuelo | 8 | `O` cerca de un planeta; `Ship::step_orbit` |
| Navegación | Aceleración, frenado y giro progresivos, iguales en cualquier equipo | 8 | Pasos fijos en `src/simulation.rs` |
| Navegación | Atracción gravitatoria que desvía la nave | 10 | `G` la apaga; `ship::gravity_at` |
| Navegación | Colisión que impide atravesar los planetas | 4 | `Ship::collide` |
| Rendimiento | Descarte de objetos fuera de la vista, con contadores | 7 | `F` lo apaga; `src/render/frustum.rs` |
| Rendimiento | Niveles de detalle según la distancia, con contadores | 8 | `L` lo apaga; `SolarSystem::sphere_level` |
| Presentación | Coherencia visual y recorrido claro (nombres de planetas, contador de descubiertos, sonido) | — | A criterio del profesor |

## Uso de IA y recursos externos

- **IA:** usamos Claude Code con el modelo Claude Opus 5.5 para escribir y revisar parte
  del código, proponer opciones para mejorar el rendimiento y la visibilidad del juego,
  corregir errores que aparecían durante el desarrollo y ordenar la documentación. Cada
  integrante revisó el código de su parte y entiende su funcionamiento.
- **Bibliotecas:** `raylib` (ventana, teclado, mostrar el framebuffer, texto y
  reproducción de audio), `noise` (ruido OpenSimplex) y `rayon` (repartir trabajo entre
  hilos).
- **Recursos:** las tres naves (`assets/nave_graficas.obj`, `assets/birdship.obj` y
  `assets/nave.obj`) las modelamos en Blender. Los objetos de
  superficie y las esferas se generan en código. No hay imágenes ni archivos de audio
  externos: el sonido se sintetiza en `src/audio/synth.rs`.

## Planetas

Cada cuerpo recibe en `ObjectContext` qué tan cerca está la nave (`proximity`, de 0 a
1) y la dirección hacia ella en su propio espacio (`ship_direction`), que en la esfera
es el punto justo debajo de la nave. Con eso los shaders cambian al acercarse, sin
cambiar de escena. Al acercarse, el HUD muestra el nombre del planeta y lleva la cuenta
de los descubiertos.

| Cuerpo | Shader | Qué pasa al acercarse |
|---|---|---|
| Sol | `shaders/sun.rs`: plasma que hierve, granulación (Worley), manchas, filamentos | Hierve más rápido, se encienden los filamentos y brilla más; el casco se calienta |
| Jungla | `shaders/jungle.rs`: mar, selva, ríos, nubes con sombra, luces de noche | Se abre un claro en las nubes bajo la nave y se avivan las luciérnagas |
| Ambar | `shaders/gas_giant.rs`: bandas con corrientes, tormenta en espiral, relámpagos | La tormenta gira más rápido y hay más relámpagos |
| Kryos | `shaders/ice.rs`: hielo agrietado (Worley), reflejo, auroras polares | Las grietas se encienden en ondas cian desde la nave; las auroras se intensifican |
| Ignis | `shaders/volcanic.rs`: placas de basalto, lava que fluye, volcanes | Las grietas se abren, salen ondas de calor y erupciona más; el casco se pone al rojo |
| Eco | `shaders/dormant.rs`: roca con red hexagonal tallada | La red se enciende hexágono por hexágono desde la nave, con pulsos y glifos |

## Cómo se ejecuta

Requiere Rust. `raylib` se compila desde su código fuente, así que también necesita
CMake, un compilador de C y las bibliotecas de desarrollo de OpenGL y del sistema de
ventanas.

```bash
cargo run --release                          # ventana
cargo run --release -- --benchmark           # ms por cuadro en poses fijas de la nave, sin ventana
cargo run --release -- --screenshot captura  # cada nave (menú, vuelo y planta), cada planeta, poses fijas
cargo test --release                         # pruebas
```

Se ejecuta desde la raíz del repositorio: los modelos se cargan de `assets/`.
`--screenshot` guarda también cada planeta con la nave cerca (`<prefijo>-<planeta>-cerca.png`)
y una vista a ras del suelo de su lado de día (`<prefijo>-<planeta>-suelo.png`).

**Windows:** si la compilación falla con `'vcruntime.h' file not found`, `bindgen` no
encuentra los headers de MSVC (pasa cuando hay una instalación de Visual Studio sin las
herramientas de C++). Se le indica la carpeta con una variable de entorno, ajustando la
versión a la que esté instalada:

```powershell
$env:BINDGEN_EXTRA_CLANG_ARGS = '-I"C:/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/14.43.34808/include"'
```

### Controles

| Tecla | Acción |
|---|---|
| `W` / `S` | Acelerar / frenar |
| Flechas ↓ / ↑ | Subir / bajar la punta, como la palanca de un avión (halar sube, empujar baja) |
| Flechas ← / → | Girar a la izquierda / derecha |
| `I` | Invertir las flechas arriba y abajo (arriba sube) |
| `A` / `D` | Alabear (girar sobre su eje) |
| `Shift` | Turbo |
| ← / → (en el menú) | Cambiar de nave |
| `Enter` (en el menú) | Despegar con la nave elegida |
| `Tab` | Volver al menú para cambiar de nave |
| `C` | Inspeccionar: orbitar alrededor de la nave con flechas y rueda; otra vez `C` para volver |
| `R` | Volver la nave al inicio |
| Espacio | Pausar la simulación |
| `L` | Niveles de detalle: por distancia o siempre el máximo |
| `F` | Descarte de objetos fuera de la vista: activo o apagado |
| `O` | Cerca de un planeta: entrar o salir del modo orbital |
| `G` | Gravedad: activa o apagada |
| `N` | Otra semilla para repartir los objetos de la superficie |
| `H` | HUD completo, mínimo u oculto (para grabar) |
| `M` | Silenciar o activar el sonido |

La aceleración y los giros son progresivos. La física avanza en pasos fijos de 1/120 s,
así que la nave responde igual sin importar cuántos cuadros por segundo dibuje el equipo.
No se puede atravesar el sol ni los planetas.

### Gravedad y modo orbital

- **Gravedad** (`ship::gravity_at`): cada cuerpo jala a la nave hacia su centro. En la
  superficie la aceleración es proporcional al radio (un planeta grande jala más) y cae
  con el cuadrado de la distancia. Si la nave pasa cerca sin motor, su trayectoria se
  curva. El HUD muestra la aceleración y qué cuerpo jala más.
- **Modo orbital** (`Ship::step_orbit`): cerca de un planeta, `O` cambia el control para
  rodearlo sin cortar el vuelo. La nave viaja con el planeta, se nivela sola con el techo
  hacia afuera y avanza a la misma altura curvándose con la superficie. W/S cambian la
  velocidad, izquierda y derecha el rumbo, y arriba y abajo la altura. Si sube demasiado
  vuelve sola al vuelo libre.
- **Partículas del turbo** (`ship/exhaust.rs`, `render/sprites.rs`): con `Shift` cada
  motor suelta chispas que se enfrían de blanco a rojo y quedan como estela. Avanzan en
  los pasos fijos de la física con una semilla fija, y el renderer las dibuja después de
  los objetos como discos de luz: prueban la profundidad sin escribirla y suman su color.

### Sonido

Todo el sonido se calcula en `src/audio/synth.rs`, muestra por muestra; `raylib` solo lo
reproduce.

- **Capa continua, en tiempo real:** un zumbido grave del espacio, el motor (tono y
  retumbe que suben con el acelerador), el siseo del turbo y chasquidos de calor junto a
  Ignis o al sol. Cada cuadro se le pasan los parámetros de la nave y, cuando raylib
  terminó de tocar un bloque de muestras, se calcula el siguiente.
- **Efectos:** acorde al descubrir un planeta, barrido de tono al entrar o salir del modo
  orbital, clic al cambiar una opción y alarma de temperatura. Se generan al arrancar y
  se cargan como WAV desde memoria.

## Naves

Al abrir el juego aparece un menú para elegir la nave. La vista previa no es una
imagen: es la nave girando, dibujada con el mismo renderer y su shader, con los motores
encendidos. ← y → cambian de nave, `Enter` despega y `Tab`, en vuelo, vuelve al menú.

| Nave | Modelo | Triángulos | Pintura | Toberas |
|---|---|---:|---|---|
| Raptor | `assets/nave_graficas.obj` | 39 168 | Blanco desgastado con franjas rojas (`ShipPaint::Classic`) | Puntas de las alas y cola |
| Comet | `assets/birdship.obj` | 8 960 | Gris azulado con acentos naranja, motores azules | Bloque de motores bajo la cola |
| Stinger | `assets/nave.obj` | 802 | Grafito con acentos amarillos, motores violeta | Motor central y cápsulas laterales |

- **Tabla de naves** (`src/ship/models.rs`): nombre, autor, archivo, toberas y pintura de
  cada una. La física, la cámara y el tamaño en el mundo son iguales para todas: cada
  modelo se escala para que quepa en la misma esfera (`SHIP_RADIUS`).
- **Pintura** (`src/shaders/ship.rs`): la iluminación y el calor del casco son comunes.
  `ShipPaint::Classic` está ajustada a la forma de la nave Raptor; `ShipPaint::Livery`
  sirve para cualquier modelo y se configura con colores y unas pocas medidas (ancho de
  la franja del lomo, dónde empiezan las puntas de las alas y detrás de qué `z` están las
  toberas).
- **Turbo:** las partículas salen de las toberas de la nave elegida.

## Superficie

Jungla, Ignis y Kryos tienen objetos sobre su superficie. Los modelos se generan en
código (`src/mesh/shapes.rs`) con caras planas, y cada tipo tiene varias formas para
que las copias no sean iguales.

| Planeta | Objeto | Dónde va | Qué lo distingue |
|---|---|---|---|
| Jungla | Árbol: tronco inclinado y copa de follaje irregular | Selva: tierra firme, sin mar, mesetas ni aldeas | Luciérnagas en la copa de noche |
| Jungla | Choza de barro y palos con techo de paja | Aldeas, donde de noche se ven las fogatas | Fuego encendido en la puerta |
| Ignis | Roca de basalto con hendiduras y poros: peñas altas o peñascos bajos | Placas firmes, lejos de la lava y de los cráteres | Lava en los poros y base al rojo |
| Kryos | Cristales de hielo: grupos de puntas o una punta grande suelta | Grietas grandes del hielo | Brillo cian por dentro que crece con la nave cerca |

- **Distribución** (`scene::surface::place`): se toman puntos al azar sobre la esfera
  con la semilla `SURFACE_SEED` (`src/config.rs`) y se quedan solo donde un ruido supera
  un umbral, para que se agrupen en manchas, y donde el terreno del shader del planeta
  lo permite, y lejos de los otros objetos de su tipo (`spacing`). Misma semilla,
  mismas posiciones; `N` cambia la semilla en vivo. Cuántos
  hay de cada tipo, su tamaño y cómo se agrupan se ajustan en el campo `surface` de cada
  planeta en `src/scene/planets.rs`.
- **Orientación** (`scene::surface::surface_frame`): cada objeto se para con su `+y`
  perpendicular al suelo y con un giro propio sobre ese eje. Su matriz es
  `planet_frame * surface_frame * escala`, así que orbita y gira con el planeta.
- **Aparición:** desde lejos no se dibujan. Al acercarse la nave brotan del suelo,
  primero los que quedan justo debajo de ella, y se esconden al alejarse. Depende de la
  cercanía, no del tiempo.
- **Shaders** (`src/shaders/props.rs`): basalto, cristal, follaje, paja y pared de
  barro. Los modelos les pasan datos por vértice en `uv` (profundidad del poro, altura
  del cristal, ángulo de la puerta).

## Estructura

```
src/
  main.rs            ventana, bucle, --benchmark y --screenshot
  menu.rs            menú de selección de naves con vista previa 3D
  config.rs          resolución, campo de visión, planos, semilla, rutas
  math/              Mat4 y Quat propios
  mesh/              Vertex, Mesh, esfera envolvente, cargador OBJ, icosfera
    shapes.rs        modelos de los objetos de superficie
  render/            cámara, contexto del cuadro, pipeline, rasterizador,
                     framebuffer, fondo, contadores
  scene/             SceneObject, Collider y el sistema solar
    planets.rs       datos de cada cuerpo: radio, órbita, giro, material, lunas, superficie
    surface.rs       reparto y orientación de los objetos de superficie
  ship/              física de la nave, cámara en tercera persona y controles
    models.rs        tabla de naves: modelo, toberas y pintura de cada una
  simulation.rs      avance del mundo en pasos fijos
  shaders/           Material, shaders y funciones de ruido
assets/
  nave_graficas.obj  nave Raptor (Blender, 39 168 triángulos)
  birdship.obj       nave Comet (Blender, 8 960 triángulos)
  nave.obj           nave Stinger (Blender, 802 triángulos)
  sphere.obj         esfera para las pruebas del cargador OBJ; la escena usa la icosfera
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
- **Unidades:** el sol mide 12 de radio; los planetas, de 4.5 a 9, y orbitan entre 60 y
  240 unidades del sol.

### Cómo agregar algo

- **Un shader:** variante nueva en `shaders::Material` con sus parámetros, archivo
  nuevo en `src/shaders/` con su función, y una rama en `shaders::shade`. Los patrones
  se calculan sobre `fragment.object_position` y `fragment.object_normal` para que se
  muevan y giren con el objeto; la iluminación usa `fragment.normal` (mundo). Para
  efectos que dependen del sol en el espacio del objeto (sombras de nubes, lado de
  noche), el shader recibe `ObjectContext::sun_direction`. Ejemplo completo:
  `src/shaders/jungle.rs`.
- **Un planeta:** agregar una entrada a `PLANETS` en `src/scene/planets.rs` con su
  radio, órbita, giro y material. Con eso ya orbita, gira, se dibuja y la nave choca con
  él. `cargo test` revisa que su órbita no choque con las vecinas.
- **La apariencia de un planeta:** se crea su shader (ver el punto anterior) y se pone en
  el campo `material` de su entrada. Ejemplos: `src/shaders/jungle.rs`, `ice.rs`,
  `volcanic.rs`.
- **Un objeto sobre un planeta:** su matriz de modelo es
  `SolarSystem::planet_frame(planeta, tiempo) * posición_local`. Así orbita y gira con el
  planeta. Para repartir muchos, se agrega un `Prop` en `src/scene/surface.rs` con su
  malla y su material, y una entrada en el campo `surface` del planeta.
- **Un objeto:** construir un `SceneObject` con su malla (`Arc<Mesh>`, compartida entre
  copias), su matriz de modelo y su material, y agregarlo a la lista del cuadro.
- **Una nave:** exportarla de Blender como OBJ con normales y la punta hacia `+z`, ponerla
  en `assets/` y agregar una entrada a `SHIPS` en `src/ship/models.rs` con sus toberas y
  su pintura (`ShipPaint::Livery`). Si quedó con la punta hacia `-z`, `turn_around: true`
  la gira. `--screenshot` guarda una vista desde arriba de cada nave
  (`<prefijo>-planta-<nave>.png`) con la punta hacia arriba, para revisarla; `cargo test`
  revisa que sus toberas queden atrás y cerca del casco.
- **Un modelo de Blender:** exportar OBJ con triángulos o polígonos y normales, ponerlo
  en `assets/` y cargarlo con `Obj::load(ruta)?.to_mesh(Normals::Authored)`.
  `Normals::Smooth` promedia normales para superficies curvas.

## Rendimiento de referencia

Ryzen 7 8845HS, 800×450, `--benchmark` (varía según el estado del equipo):

| Pose de la nave | ms por cuadro |
|---|---:|
| Inicio | 5.0 |
| Junto a la superficie de Jungla | 22.5 |
| Junto al sol | 8.5 |

## Esferas, niveles de detalle y descarte

- **Icosfera** (`src/mesh/icosphere.rs`): el sol, los planetas y la luna son esferas
  generadas antes de rasterizar. Se parte de un icosaedro y cada nivel parte cada
  triángulo en cuatro, empujando los puntos medios a la superficie: 20, 80, 320, 1 280,
  5 120 y 20 480 triángulos.
- **Niveles de detalle** (`SolarSystem::sphere_level`): cada cuadro, cada esfera usa el
  nivel con el que sus triángulos miden unos 6 píxeles en pantalla. Un planeta lejano
  usa 80 triángulos; uno que llena la pantalla, 20 480.
- **Descarte** (`src/render/frustum.rs`): antes de transformar los vértices de un
  objeto, se prueba su esfera envolvente contra los seis planos de la vista. Si queda
  fuera, el objeto se salta entero.

- **Objetos pequeños** (`render/pipeline.rs`): el trabajo de un objeto se reparte entre
  hilos solo si tiene suficientes vértices o fragmentos. Con cientos de objetos de
  superficie de pocos triángulos, coordinar los hilos costaba más que dibujarlos.

El HUD muestra los contadores de objetos y triángulos, y `L` y `F` apagan cada
optimización para compararlas en vivo. `--benchmark` mide cada pose con y sin ellas.
Triángulos enviados al pipeline:

| Pose de la nave | Con niveles de detalle y descarte | Sin optimizar |
|---|---:|---:|
| Inicio | 41 248 (6 de 8 objetos) | 182 528 |
| Junto a la superficie de Jungla | 66 208 | 182 528 |
| Junto al sol | 62 368 | 182 528 |
