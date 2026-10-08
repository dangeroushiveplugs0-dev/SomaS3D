#[derive(Debug, Clone, PartialEq)]
pub struct PbrMaterial {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub ambient_occlusion: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub opacity: f32,
}

impl Default for PbrMaterial {
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            normal_scale: 1.0,
            ambient_occlusion: 1.0,
            emissive: [0.0, 0.0, 0.0],
            emissive_strength: 0.0,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialSemantics {
    /// Higher values represent a more water-coated surface.
    pub wetness: f32,
    /// Higher values represent a deliberately dry/chalky surface response.
    pub dryness: f32,
    /// Semantic authoring control for organic-style material recipes.
    pub organicness: f32,
}

impl Default for MaterialSemantics {
    fn default() -> Self {
        Self {
            wetness: 0.0,
            dryness: 0.0,
            organicness: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub name: String,
    pub pbr: PbrMaterial,
    pub semantics: MaterialSemantics,
}
