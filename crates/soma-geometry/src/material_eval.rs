use super::Material;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvaluatedPbr {
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub normal_scale: f32,
    pub ambient_occlusion: f32,
    pub emissive: [f32; 3],
    pub emissive_strength: f32,
    pub opacity: f32,
}

/// Converts SomaS3D's semantic material controls into ordinary PBR inputs.
///
/// This is deliberately deterministic and bounded. Future renderer-specific
/// features may add richer water films, microstructure, or subsurface models,
/// but the authoring semantics remain independent from the renderer.
pub fn evaluate(material: &Material) -> EvaluatedPbr {
    let p = &material.pbr;
    let s = &material.semantics;

    let wet = s.wetness.clamp(0.0, 1.0);
    let dry = s.dryness.clamp(0.0, 1.0);
    let roughness = (p.roughness * (1.0 - 0.65 * wet) + 0.20 * dry).clamp(0.0, 1.0);

    EvaluatedPbr {
        base_color: p.base_color,
        metallic: p.metallic.clamp(0.0, 1.0),
        roughness,
        normal_scale: p.normal_scale.max(0.0),
        ambient_occlusion: p.ambient_occlusion.clamp(0.0, 1.0),
        emissive: p.emissive,
        emissive_strength: p.emissive_strength.max(0.0),
        opacity: p.opacity.clamp(0.0, 1.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Material, MaterialSemantics, PbrMaterial};

    #[test]
    fn wetness_lowers_effective_roughness() {
        let mut material = Material {
            name: "test".into(),
            pbr: PbrMaterial::default(),
            semantics: MaterialSemantics::default(),
        };
        let dry = evaluate(&material).roughness;
        material.semantics.wetness = 1.0;
        assert!(evaluate(&material).roughness < dry);
    }

    #[test]
    fn dryness_cannot_push_roughness_above_one() {
        let material = Material {
            name: "test".into(),
            pbr: PbrMaterial {
                roughness: 1.0,
                ..PbrMaterial::default()
            },
            semantics: MaterialSemantics {
                dryness: 1.0,
                ..MaterialSemantics::default()
            },
        };
        assert_eq!(evaluate(&material).roughness, 1.0);
    }
}
