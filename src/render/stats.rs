/// Contadores de trabajo del renderer en un cuadro. Sirven para demostrar que
/// el culling y los niveles de detalle cambian lo que hace el renderer.
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderStats {
    pub objects_total: usize,
    pub objects_drawn: usize,
    /// Descartados enteros por quedar fuera de la vista.
    pub objects_culled: usize,
    /// Triángulos que entraron al pipeline.
    pub triangles_submitted: usize,
    /// Descartados por quedar completos detrás del plano cercano.
    pub triangles_behind_camera: usize,
    /// Cruzaban el plano cercano y se recortaron.
    pub triangles_clipped: usize,
    /// Descartados por dar la espalda a la cámara o por ser tan pequeños en
    /// pantalla que su área queda en cero.
    pub triangles_backfacing: usize,
    /// Llegaron a recorrerse píxel por píxel. Un triángulo recortado puede
    /// aportar dos.
    pub triangles_rasterized: usize,
    /// Fragmentos producidos por la rasterización.
    pub fragments_rasterized: usize,
    /// Fragmentos que pasaron la prueba de profundidad y se sombrearon.
    pub fragments_shaded: usize,
    /// Partículas que llegaron a pintar algún píxel.
    pub sprites_drawn: usize,
}

impl RenderStats {
    /// Texto para el HUD, una línea por renglón.
    pub fn lines(&self) -> [String; 3] {
        [
            format!(
                "objetos: {}/{} dibujados ({} fuera de vista)",
                self.objects_drawn, self.objects_total, self.objects_culled
            ),
            format!(
                "triangulos: {} enviados, {} rasterizados ({} espalda/sin area, {} detras, {} recortados)",
                self.triangles_submitted,
                self.triangles_rasterized,
                self.triangles_backfacing,
                self.triangles_behind_camera,
                self.triangles_clipped,
            ),
            format!(
                "fragmentos: {} rasterizados, {} sombreados; particulas: {}",
                self.fragments_rasterized, self.fragments_shaded, self.sprites_drawn
            ),
        ]
    }
}
