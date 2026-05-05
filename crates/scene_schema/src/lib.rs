use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const SCHEMA_VERSION: &str = "0.1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneFile {
    pub version: String,
    pub document: SceneDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneDocument {
    pub id: String,
    pub name: String,
    pub width: f64,
    pub height: f64,
    pub background: DocumentBackground,
    pub resources: DocumentResources,
    pub root: SceneNode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DocumentBackground {
    #[serde(rename = "type")]
    pub background_type: BackgroundType,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundType {
    Solid,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DocumentResources {
    #[serde(default)]
    pub images: HashMap<String, Value>,
    #[serde(default)]
    pub fonts: HashMap<String, Value>,
    #[serde(default)]
    pub palettes: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: NodeType,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    #[serde(default)]
    pub blend_mode: BlendMode,
    pub transform: Transform,
    #[serde(default)]
    pub params: JsonObject,
    #[serde(default)]
    pub style: JsonObject,
    #[serde(default)]
    pub children: Vec<SceneNode>,
    #[serde(default)]
    pub meta: JsonObject,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    Group,
    Rectangle,
    Ellipse,
    Path,
    Text,
    ImageLayer,
    Shadow,
    Blur,
}

impl NodeType {
    pub fn can_have_children(self) -> bool {
        matches!(self, Self::Group)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "lowercase")]
pub enum BlendMode {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    #[serde(rename = "scaleX")]
    pub scale_x: f64,
    #[serde(rename = "scaleY")]
    pub scale_y: f64,
    pub rotation: f64,
    pub opacity: f64,
}

pub type JsonObject = Map<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl ValidationIssue {
    fn new(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            message: message.into(),
        }
    }
}

pub fn parse_scene_str(input: &str) -> Result<SceneFile, serde_json::Error> {
    serde_json::from_str(input)
}

pub fn validate_scene(scene: &SceneFile) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    if scene.version != SCHEMA_VERSION {
        issues.push(ValidationIssue::new(
            "version",
            format!(
                "expected schema version {SCHEMA_VERSION}, found {}",
                scene.version
            ),
        ));
    }

    if scene.document.width <= 0.0 {
        issues.push(ValidationIssue::new(
            "document.width",
            "document width must be greater than zero",
        ));
    }

    if scene.document.height <= 0.0 {
        issues.push(ValidationIssue::new(
            "document.height",
            "document height must be greater than zero",
        ));
    }

    if scene.document.root.node_type != NodeType::Group {
        issues.push(ValidationIssue::new(
            "document.root.type",
            "root node must be of type Group",
        ));
    }

    let mut ids = HashSet::new();
    validate_node(&scene.document.root, "document.root", &mut ids, &mut issues);

    issues
}

fn validate_node(
    node: &SceneNode,
    path: &str,
    ids: &mut HashSet<String>,
    issues: &mut Vec<ValidationIssue>,
) {
    if node.id.trim().is_empty() {
        issues.push(ValidationIssue::new(
            format!("{path}.id"),
            "node id must not be empty",
        ));
    } else if !ids.insert(node.id.clone()) {
        issues.push(ValidationIssue::new(
            format!("{path}.id"),
            format!("duplicate node id {}", node.id),
        ));
    }

    if node.name.trim().is_empty() {
        issues.push(ValidationIssue::new(
            format!("{path}.name"),
            "node name must not be empty",
        ));
    }

    if !(0.0..=1.0).contains(&node.transform.opacity) {
        issues.push(ValidationIssue::new(
            format!("{path}.transform.opacity"),
            "opacity must be between 0 and 1",
        ));
    }

    for (field, value) in [
        ("x", node.transform.x),
        ("y", node.transform.y),
        ("scaleX", node.transform.scale_x),
        ("scaleY", node.transform.scale_y),
        ("rotation", node.transform.rotation),
    ] {
        if !value.is_finite() {
            issues.push(ValidationIssue::new(
                format!("{path}.transform.{field}"),
                "transform values must be finite numbers",
            ));
        }
    }

    if !node.node_type.can_have_children() && !node.children.is_empty() {
        issues.push(ValidationIssue::new(
            format!("{path}.children"),
            format!("nodes of type {:?} cannot have children", node.node_type),
        ));
    }

    for (index, child) in node.children.iter().enumerate() {
        validate_node(child, &format!("{path}.children[{index}]"), ids, issues);
    }
}

#[cfg(test)]
mod tests {
    use super::{NodeType, SCHEMA_VERSION, parse_scene_str, validate_scene};

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
    const SHAPES_STUDY: &str = include_str!("../../../examples/shapes_study.vsd.json");
    const HYBRID_SCENE: &str = include_str!("../../../examples/hybrid_scene.vsd.json");

    #[test]
    fn parses_example_documents() {
        for input in [BASIC_POSTER, SHAPES_STUDY, HYBRID_SCENE] {
            let parsed = parse_scene_str(input).expect("example scene should parse");
            assert_eq!(parsed.version, SCHEMA_VERSION);
        }
    }

    #[test]
    fn validates_example_documents() {
        for input in [BASIC_POSTER, SHAPES_STUDY, HYBRID_SCENE] {
            let parsed = parse_scene_str(input).expect("example scene should parse");
            let issues = validate_scene(&parsed);
            assert!(
                issues.is_empty(),
                "expected no validation issues, found: {issues:?}"
            );
        }
    }

    #[test]
    fn rejects_non_group_root() {
        let mut parsed = parse_scene_str(BASIC_POSTER).expect("example scene should parse");
        parsed.document.root.node_type = NodeType::Rectangle;

        let issues = validate_scene(&parsed);
        assert!(
            issues
                .iter()
                .any(|issue| issue.path == "document.root.type")
        );
    }

    #[test]
    fn rejects_duplicate_ids() {
        let mut parsed = parse_scene_str(BASIC_POSTER).expect("example scene should parse");
        parsed.document.root.children[0].id = parsed.document.root.id.clone();

        let issues = validate_scene(&parsed);
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("duplicate node id"))
        );
    }
}
