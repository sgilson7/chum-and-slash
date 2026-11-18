//! Characters built from primitives on a two-level joint rig (CS-20, M1.2).
//!
//! A model is a tree of joints, each placed in its parent's space and turned
//! by three angles from a pose, and a list of parts, each a primitive mesh
//! hung on a joint. Protection of the Spheres' heroes hang their parts on one
//! hinge per joint (`refs/pots.md`); this rig adds a second level (shoulder
//! then elbow, hip then knee) and three angles a joint, so a clip can bend an
//! arm or a leg. The data is `data/models.json`, in whole millimetres,
//! degrees and 0-255 colours, so no float is parsed (D2).

use crate::fx::{self, ONE};
use crate::geom::{v3, Affine, V3};
use crate::mesh::Mesh;
use crate::render::Rgb;
use serde::Deserialize;
use std::collections::BTreeMap;

pub const MODELS_JSON: &str = include_str!("../../../data/models.json");

/// Shader styles a part can carry (the colour's fourth channel). The
/// numbers are the shader's own; `uniforms()` sends them.
pub const STYLE_PLAIN: i32 = 0;
/// Drawn unlit: the utility tool's glowing edge.
pub const STYLE_GLOW: i32 = 1;
/// Vertical wales round the part: the hero's corduroy (CS-20).
pub const STYLE_STRIPES: i32 = 2;

#[derive(Debug, Clone, Deserialize)]
struct JointSpec {
    name: String,
    parent: Option<String>,
    /// where the joint sits in its parent's space, mm
    at: [i32; 3],
}

#[derive(Debug, Clone, Deserialize)]
struct PartSpec {
    joint: String,
    mesh: String,
    /// centre in the joint's space, mm
    at: [i32; 3],
    /// size, mm
    size: [i32; 3],
    /// a fixed turn of the part in its joint, degrees (x, y, z)
    #[serde(default)]
    turn: [i32; 3],
    colour: String,
    #[serde(default)]
    style: String,
    /// a name tests and the harness can ask for ("trousers", "coat")
    #[serde(default)]
    role: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelSpec {
    joints: Vec<JointSpec>,
    parts: Vec<PartSpec>,
}

#[derive(Debug, Clone, Deserialize)]
struct ModelsFile {
    colours: BTreeMap<String, [i32; 3]>,
    models: BTreeMap<String, ModelSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    pub at: V3,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub joint: usize,
    pub mesh: Mesh,
    pub at: V3,
    pub size: V3,
    pub turn: [i32; 3],
    pub rgb: Rgb,
    /// the colour's name in `data/models.json`, for tests and the harness
    pub colour: String,
    pub style: i32,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub name: String,
    pub joints: Vec<Joint>,
    pub parts: Vec<Part>,
}

/// A pose: three angles (x, y, z, in angle units) for each joint, in the
/// model's joint order.
pub type Pose = Vec<[i32; 3]>;

pub fn mm(n: i32) -> i32 {
    ((n as i64 * ONE as i64) / 1000) as i32
}

pub fn mm3(a: [i32; 3]) -> V3 {
    v3(mm(a[0]), mm(a[1]), mm(a[2]))
}

pub fn deg3(a: [i32; 3]) -> [i32; 3] {
    [fx::deg(a[0]), fx::deg(a[1]), fx::deg(a[2])]
}

pub fn rgb255(c: [i32; 3]) -> Rgb {
    [c[0] * ONE / 255, c[1] * ONE / 255, c[2] * ONE / 255]
}

fn mesh_named(s: &str) -> Result<Mesh, String> {
    Ok(match s {
        "cube" => Mesh::Cube,
        "wedge" => Mesh::Wedge,
        "octa" => Mesh::Octa,
        "sphere" => Mesh::Sphere,
        "cyl" => Mesh::Cyl,
        "cone" => Mesh::Cone,
        "taper" => Mesh::Taper,
        "blob" => Mesh::Blob,
        "facet" => Mesh::Facet,
        "facetcyl" => Mesh::FacetCyl,
        "facetcone" => Mesh::FacetCone,
        other => return Err(format!("no mesh called {other}")),
    })
}

fn style_named(s: &str) -> Result<i32, String> {
    Ok(match s {
        "" | "plain" => STYLE_PLAIN,
        "glow" => STYLE_GLOW,
        "stripes" => STYLE_STRIPES,
        other => return Err(format!("no style called {other}")),
    })
}

/// Every model in `data/models.json`, checked: each joint's parent comes
/// before it, and each part names a joint, a mesh and a colour that exist.
pub fn load(json: &str) -> Result<BTreeMap<String, Model>, String> {
    let file: ModelsFile = serde_json::from_str(json).map_err(|e| format!("models.json: {e}"))?;
    let mut out = BTreeMap::new();
    for (name, spec) in file.models {
        let mut joints: Vec<Joint> = Vec::new();
        for j in &spec.joints {
            let parent = match &j.parent {
                None => None,
                Some(p) => Some(joints.iter().position(|k| &k.name == p).ok_or(format!(
                    "{name}: joint {} names parent {p}, not defined above it",
                    j.name
                ))?),
            };
            joints.push(Joint {
                name: j.name.clone(),
                parent,
                at: mm3(j.at),
            });
        }
        let mut parts = Vec::new();
        for p in &spec.parts {
            let joint = joints.iter().position(|k| k.name == p.joint).ok_or(format!(
                "{name}: a part hangs on joint {}, which is missing",
                p.joint
            ))?;
            let c = file
                .colours
                .get(&p.colour)
                .ok_or(format!("{name}: no colour called {}", p.colour))?;
            parts.push(Part {
                joint,
                mesh: mesh_named(&p.mesh)?,
                at: mm3(p.at),
                size: mm3(p.size),
                turn: deg3(p.turn),
                rgb: rgb255(*c),
                colour: p.colour.clone(),
                style: style_named(&p.style)?,
                role: p.role.clone(),
            });
        }
        out.insert(name.clone(), Model { name, joints, parts });
    }
    Ok(out)
}

/// The models shipped with the game.
pub fn shipped() -> BTreeMap<String, Model> {
    load(MODELS_JSON).unwrap_or_default()
}

fn turned(a: [i32; 3]) -> Affine {
    Affine::rot_y(a[1])
        .then(&Affine::rot_x(a[0]))
        .then(&Affine::rot_z(a[2]))
}

impl Model {
    pub fn joint(&self, name: &str) -> Option<usize> {
        self.joints.iter().position(|j| j.name == name)
    }

    pub fn rest(&self) -> Pose {
        vec![[0; 3]; self.joints.len()]
    }

    /// Each joint's placement in the model's space for a pose. A parent is
    /// always placed before its children (`load` checks the order).
    pub fn placed(&self, pose: &Pose) -> Vec<Affine> {
        let mut out: Vec<Affine> = Vec::with_capacity(self.joints.len());
        for (i, j) in self.joints.iter().enumerate() {
            let local = Affine::translate(j.at).then(&turned(pose.get(i).copied().unwrap_or([0; 3])));
            let a = match j.parent {
                Some(p) => out[p].then(&local),
                None => local,
            };
            out.push(a);
        }
        out
    }

    /// Each part's placement in the world, given where the model stands
    /// (`root`, its feet, and the yaw it faces) and a pose. The model can be
    /// grown by `scale` (Q16; the large dog is a dog at 2.2).
    pub fn parts_at(&self, root: &Affine, pose: &Pose, scale: i32) -> Vec<(usize, Affine)> {
        let placed = self.placed(pose);
        let grow = Affine::scale(v3(scale, scale, scale));
        self.parts
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let a = root
                    .then(&grow)
                    .then(&placed[p.joint])
                    .then(&Affine::translate(p.at))
                    .then(&turned(p.turn))
                    .then(&Affine::scale(p.size));
                (i, a)
            })
            .collect()
    }

    /// Where a joint is in the world for a pose (the tool's tip, the
    /// dog's jaw): its origin.
    pub fn joint_at(&self, root: &Affine, pose: &Pose, scale: i32, joint: usize) -> V3 {
        let grow = Affine::scale(v3(scale, scale, scale));
        root.then(&grow).then(&self.placed(pose)[joint]).apply(V3::ZERO)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hero() -> Model {
        shipped().remove("hero").expect("the hero is in models.json")
    }

    fn colours_of(m: &Model, role: &str) -> Vec<String> {
        let mut c: Vec<String> = m
            .parts
            .iter()
            .filter(|p| p.role == role)
            .map(|p| p.colour.clone())
            .collect();
        c.sort();
        c.dedup();
        c
    }

    #[test]
    fn shipped_models_load() {
        let all = load(MODELS_JSON).expect("models.json loads");
        for name in ["hero", "dog"] {
            assert!(all.contains_key(name), "{name} is missing");
        }
    }

    /// CS-20: black shirt, green coat, cuffed corduroy trousers striped
    /// white and brown about every centimetre, a bladed tool that glows.
    #[test]
    fn hero_model_has_the_described_parts_and_colours() {
        let h = hero();
        assert_eq!(colours_of(&h, "shirt"), ["black"]);
        assert_eq!(colours_of(&h, "coat"), ["coat_green"]);
        let trousers: Vec<&Part> = h.parts.iter().filter(|p| p.role == "trousers").collect();
        assert_eq!(trousers.len(), 4, "two thighs and two shins");
        for t in &trousers {
            assert_eq!(t.style, STYLE_STRIPES, "the trousers carry the corduroy stripes");
            assert_eq!(
                t.colour, "cord_white",
                "white wales, brown between them (uniforms)"
            );
        }
        assert_eq!(
            h.parts.iter().filter(|p| p.role == "cuff").count(),
            2,
            "both legs cuffed"
        );
        assert!(h.parts.iter().any(|p| p.role == "blade"), "the tool has a blade");
        assert!(h.parts.iter().any(|p| p.role == "pick"), "and a pick");
        assert!(
            h.parts
                .iter()
                .any(|p| p.role == "tool_edge" && p.style == STYLE_GLOW),
            "and it glows"
        );
    }

    #[test]
    fn a_parent_comes_before_its_child() {
        let bad = r#"{"colours":{"a":[0,0,0]},"models":{"m":{"joints":[
            {"name":"elbow","parent":"shoulder","at":[0,0,0]},{"name":"shoulder","parent":null,"at":[0,0,0]}],"parts":[]}}}"#;
        assert!(load(bad).unwrap_err().contains("not defined above it"));
    }

    /// Bending the elbow moves the hand but not the shoulder: the rig has a
    /// second level (POTS's had one hinge per limb).
    #[test]
    fn the_elbow_bends_the_forearm_only() {
        let h = hero();
        let (sh, el, tool) = (
            h.joint("r_shoulder").unwrap(),
            h.joint("r_elbow").unwrap(),
            h.joint("tool").unwrap(),
        );
        let root = Affine::translate(V3::ZERO);
        let rest = h.rest();
        let mut bent = h.rest();
        bent[el] = [fx::deg(-90), 0, 0];
        assert_eq!(
            h.joint_at(&root, &rest, ONE, sh),
            h.joint_at(&root, &bent, ONE, sh)
        );
        assert_eq!(
            h.joint_at(&root, &rest, ONE, el),
            h.joint_at(&root, &bent, ONE, el)
        );
        assert_ne!(
            h.joint_at(&root, &rest, ONE, tool),
            h.joint_at(&root, &bent, ONE, tool)
        );
    }
}
