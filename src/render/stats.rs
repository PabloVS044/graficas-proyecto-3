/// Contadores de trabajo del renderer en un cuadro. Sirven para demostrar que
/// el culling y los niveles de detalle cambian lo que hace el renderer.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub objects_total: usize,
    pub objects_drawn: usize,
    /// Triángulos que entraron al pipeline.
    pub triangles_submitted: usize,
    /// Descartados por tener un vértice detrás de la cámara.
    pub triangles_behind_camera: usize,
    /// Descartados por dar la espalda a la cámara o por ser tan pequeños en
    /// pantalla que su área queda en cero.
    pub triangles_backfacing: usize,
    /// Llegaron a recorrerse píxel por píxel.
    pub triangles_rasterized: usize,
    /// Fragmentos producidos por la rasterización.
    pub fragments_rasterized: usize,
    /// Fragmentos que pasaron la prueba de profundidad y se sombrearon.
    pub fragments_shaded: usize,
}

impl RenderStats {
    /// Texto para el HUD, una línea por renglón.
    pub fn lines(&self) -> [String; 3] {
        [
            format!("objetos: {}/{}", self.objects_drawn, self.objects_total),
            format!(
                "triangulos: {} enviados, {} rasterizados ({} espalda/sin area, {} detras)",
                self.triangles_submitted,
                self.triangles_rasterized,
                self.triangles_backfacing,
                self.triangles_behind_camera,
            ),
            format!(
                "fragmentos: {} rasterizados, {} sombreados",
                self.fragments_rasterized, self.fragments_shaded
            ),
        ]
    }
}
