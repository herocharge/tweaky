use std::collections::HashMap;

use scene_schema::{
    JsonObject, PathPoint, SceneFile, SceneNode, Transform, ValidationIssue, validate_scene,
};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn from_origin_size(origin: Point, width: f64, height: f64) -> Self {
        Self {
            x: origin.x,
            y: origin.y,
            width,
            height,
        }
    }

    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.x + self.width
            && point.y >= self.y
            && point.y <= self.y + self.height
    }

    pub fn union(self, other: Self) -> Self {
        let min_x = self.x.min(other.x);
        let min_y = self.y.min(other.y);
        let max_x = (self.x + self.width).max(other.x + other.width);
        let max_y = (self.y + self.height).max(other.y + other.height);

        Self {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentDefinition {
    pub display_name: &'static str,
    pub can_have_children: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ComponentRegistry {
    definitions: HashMap<scene_schema::NodeType, ComponentDefinition>,
}

impl ComponentRegistry {
    pub fn mvp() -> Self {
        use scene_schema::NodeType::{
            Blur, Ellipse, Group, ImageLayer, Path, Rectangle, Shadow, Text,
        };

        let mut definitions = HashMap::new();
        definitions.insert(
            Group,
            ComponentDefinition {
                display_name: "Group",
                can_have_children: true,
            },
        );
        definitions.insert(
            Rectangle,
            ComponentDefinition {
                display_name: "Rectangle",
                can_have_children: false,
            },
        );
        definitions.insert(
            Ellipse,
            ComponentDefinition {
                display_name: "Ellipse",
                can_have_children: false,
            },
        );
        definitions.insert(
            Path,
            ComponentDefinition {
                display_name: "Path",
                can_have_children: false,
            },
        );
        definitions.insert(
            Text,
            ComponentDefinition {
                display_name: "Text",
                can_have_children: false,
            },
        );
        definitions.insert(
            ImageLayer,
            ComponentDefinition {
                display_name: "Image Layer",
                can_have_children: false,
            },
        );
        definitions.insert(
            Shadow,
            ComponentDefinition {
                display_name: "Shadow",
                can_have_children: false,
            },
        );
        definitions.insert(
            Blur,
            ComponentDefinition {
                display_name: "Blur",
                can_have_children: false,
            },
        );

        Self { definitions }
    }

    pub fn definition(&self, node_type: scene_schema::NodeType) -> Option<&ComponentDefinition> {
        self.definitions.get(&node_type)
    }

    pub fn contains(&self, node_type: scene_schema::NodeType) -> bool {
        self.definitions.contains_key(&node_type)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeDocument {
    scene: SceneFile,
    registry: ComponentRegistry,
}

impl RuntimeDocument {
    pub fn new(scene: SceneFile, registry: ComponentRegistry) -> Result<Self, Vec<RuntimeIssue>> {
        let mut issues = validate_scene(&scene)
            .into_iter()
            .map(RuntimeIssue::from_validation)
            .collect::<Vec<_>>();
        issues.extend(validate_registry_compatibility(&scene, &registry));

        if issues.is_empty() {
            Ok(Self { scene, registry })
        } else {
            Err(issues)
        }
    }

    pub fn scene(&self) -> &SceneFile {
        &self.scene
    }

    pub fn registry(&self) -> &ComponentRegistry {
        &self.registry
    }

    pub fn find_node(&self, id: &str) -> Option<&SceneNode> {
        find_node(&self.scene.document.root, id)
    }

    pub fn visit_depth_first<'a>(&'a self, mut visitor: impl FnMut(NodeVisit<'a>)) {
        visit_depth_first(&self.scene.document.root, None, 0, &mut visitor);
    }

    pub fn apply(&mut self, command: DocumentCommand) -> Result<(), CommandError> {
        match command {
            DocumentCommand::RenameNode { node_id, new_name } => {
                let node = find_node_mut(&mut self.scene.document.root, &node_id)
                    .ok_or_else(|| CommandError::node_not_found(node_id.clone()))?;
                node.name = new_name;
                Ok(())
            }
            DocumentCommand::SetNodeVisibility { node_id, visible } => {
                let node = find_node_mut(&mut self.scene.document.root, &node_id)
                    .ok_or_else(|| CommandError::node_not_found(node_id.clone()))?;
                node.visible = visible;
                Ok(())
            }
            DocumentCommand::SetNodeTransform { node_id, transform } => {
                let node = find_node_mut(&mut self.scene.document.root, &node_id)
                    .ok_or_else(|| CommandError::node_not_found(node_id.clone()))?;
                node.transform = transform;
                Ok(())
            }
            DocumentCommand::SetNodeParamString {
                node_id,
                key,
                value,
            } => {
                let node = find_node_mut(&mut self.scene.document.root, &node_id)
                    .ok_or_else(|| CommandError::node_not_found(node_id.clone()))?;
                set_object_string(&mut node.params, &key, value);
                Ok(())
            }
            DocumentCommand::SetNodeStyleString {
                node_id,
                key,
                value,
            } => {
                let node = find_node_mut(&mut self.scene.document.root, &node_id)
                    .ok_or_else(|| CommandError::node_not_found(node_id.clone()))?;
                set_object_string(&mut node.style, &key, value);
                Ok(())
            }
            DocumentCommand::InsertChild {
                parent_id,
                child,
                index,
            } => {
                validate_insert_child(&self.scene.document.root, &parent_id, &child)?;
                let parent = find_node_mut(&mut self.scene.document.root, &parent_id)
                    .ok_or_else(|| CommandError::node_not_found(parent_id.clone()))?;

                let insert_index = index.unwrap_or(parent.children.len());
                if insert_index > parent.children.len() {
                    return Err(CommandError::invalid_index(
                        insert_index,
                        parent.children.len(),
                    ));
                }
                parent.children.insert(insert_index, child);
                Ok(())
            }
            DocumentCommand::RemoveNode { node_id } => {
                if self.scene.document.root.id == node_id {
                    return Err(CommandError::CannotRemoveRoot);
                }

                remove_node(&mut self.scene.document.root, &node_id)
                    .map(|_| ())
                    .ok_or_else(|| CommandError::node_not_found(node_id))
            }
        }
    }

    pub fn node_bounds(&self, node_id: &str) -> Option<Rect> {
        let node = self.find_node(node_id)?;
        bounds_for_node(node)
    }

    pub fn hit_test(&self, point: Point) -> Vec<String> {
        let mut hits = Vec::new();
        hit_test_node(&self.scene.document.root, point, &mut hits);
        hits
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentCommand {
    RenameNode {
        node_id: String,
        new_name: String,
    },
    SetNodeVisibility {
        node_id: String,
        visible: bool,
    },
    SetNodeTransform {
        node_id: String,
        transform: Transform,
    },
    SetNodeParamString {
        node_id: String,
        key: String,
        value: String,
    },
    SetNodeStyleString {
        node_id: String,
        key: String,
        value: String,
    },
    InsertChild {
        parent_id: String,
        child: SceneNode,
        index: Option<usize>,
    },
    RemoveNode {
        node_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    NodeNotFound { node_id: String },
    CannotRemoveRoot,
    InvalidChildParent { parent_id: String },
    DuplicateNodeId { node_id: String },
    InvalidInsertIndex { index: usize, len: usize },
}

impl CommandError {
    fn node_not_found(node_id: String) -> Self {
        Self::NodeNotFound { node_id }
    }

    fn invalid_index(index: usize, len: usize) -> Self {
        Self::InvalidInsertIndex { index, len }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIssue {
    pub path: String,
    pub message: String,
}

impl RuntimeIssue {
    fn from_validation(issue: ValidationIssue) -> Self {
        Self {
            path: issue.path,
            message: issue.message,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NodeVisit<'a> {
    pub depth: usize,
    pub parent_id: Option<&'a str>,
    pub node: &'a SceneNode,
}

pub fn validate_registry_compatibility(
    scene: &SceneFile,
    registry: &ComponentRegistry,
) -> Vec<RuntimeIssue> {
    let mut issues = Vec::new();
    visit_depth_first(&scene.document.root, None, 0, &mut |visit| {
        if let Some(definition) = registry.definition(visit.node.node_type) {
            if !definition.can_have_children && !visit.node.children.is_empty() {
                issues.push(RuntimeIssue {
                    path: format!("node:{}", visit.node.id),
                    message: format!(
                        "component {} does not permit children",
                        definition.display_name
                    ),
                });
            }
        } else {
            issues.push(RuntimeIssue {
                path: format!("node:{}", visit.node.id),
                message: format!("unregistered node type {:?}", visit.node.node_type),
            });
        }
    });
    issues
}

pub fn find_node<'a>(node: &'a SceneNode, id: &str) -> Option<&'a SceneNode> {
    if node.id == id {
        return Some(node);
    }

    node.children.iter().find_map(|child| find_node(child, id))
}

pub fn find_node_mut<'a>(node: &'a mut SceneNode, id: &str) -> Option<&'a mut SceneNode> {
    if node.id == id {
        return Some(node);
    }

    for child in &mut node.children {
        if let Some(found) = find_node_mut(child, id) {
            return Some(found);
        }
    }

    None
}

pub fn visit_depth_first<'a>(
    node: &'a SceneNode,
    parent_id: Option<&'a str>,
    depth: usize,
    visitor: &mut impl FnMut(NodeVisit<'a>),
) {
    visitor(NodeVisit {
        depth,
        parent_id,
        node,
    });

    for child in &node.children {
        visit_depth_first(child, Some(node.id.as_str()), depth + 1, visitor);
    }
}

pub fn bounds_for_node(node: &SceneNode) -> Option<Rect> {
    let base = match node.node_type {
        scene_schema::NodeType::Group => {
            let mut iter = node.children.iter().filter_map(bounds_for_node);
            let first = iter.next()?;
            Some(iter.fold(first, Rect::union))
        }
        scene_schema::NodeType::Rectangle => {
            let params = node.rectangle_params()?;
            Some(transformed_rect(
                &node.transform,
                params.width,
                params.height,
            ))
        }
        scene_schema::NodeType::Ellipse => {
            let params = node.ellipse_params()?;
            Some(transformed_rect(
                &node.transform,
                params.radius_x * 2.0,
                params.radius_y * 2.0,
            ))
        }
        scene_schema::NodeType::Path => {
            let params = node.path_params()?;
            bounds_from_points(&node.transform, &params.points)
        }
        scene_schema::NodeType::Text => {
            let params = node.text_params()?;
            Some(estimate_text_bounds(
                &node.transform,
                &params.text,
                params.font_size,
            ))
        }
        scene_schema::NodeType::ImageLayer => {
            let params = node.image_layer_params()?;
            Some(transformed_rect(
                &node.transform,
                params.display_width,
                params.display_height,
            ))
        }
        scene_schema::NodeType::Shadow | scene_schema::NodeType::Blur => None,
    }?;

    Some(expand_bounds_for_effects(node, base))
}

pub fn contains_point_for_node(node: &SceneNode, point: Point) -> bool {
    match node.node_type {
        scene_schema::NodeType::Path => {
            let Some(params) = node.path_params() else {
                return false;
            };
            let transformed_points = params
                .points
                .iter()
                .map(|path_point| transform_point(&node.transform, path_point.x, path_point.y))
                .collect::<Vec<_>>();

            if transformed_points.len() < 3 {
                return false;
            }

            if params.closed {
                point_in_polygon(point, &transformed_points)
            } else {
                bounds_from_points(&node.transform, &params.points)
                    .map(|bounds| bounds.contains(point))
                    .unwrap_or(false)
            }
        }
        _ => bounds_for_node(node)
            .map(|bounds| bounds.contains(point))
            .unwrap_or(false),
    }
}

fn remove_node(node: &mut SceneNode, target_id: &str) -> Option<SceneNode> {
    if let Some(index) = node.children.iter().position(|child| child.id == target_id) {
        return Some(node.children.remove(index));
    }

    for child in &mut node.children {
        if let Some(removed) = remove_node(child, target_id) {
            return Some(removed);
        }
    }

    None
}

fn validate_insert_child(
    root: &SceneNode,
    parent_id: &str,
    child: &SceneNode,
) -> Result<(), CommandError> {
    let parent = find_node(root, parent_id)
        .ok_or_else(|| CommandError::node_not_found(parent_id.to_string()))?;

    if !parent.node_type.can_have_children() {
        return Err(CommandError::InvalidChildParent {
            parent_id: parent_id.to_string(),
        });
    }

    if find_node(root, &child.id).is_some() {
        return Err(CommandError::DuplicateNodeId {
            node_id: child.id.clone(),
        });
    }

    Ok(())
}

fn transformed_rect(transform: &Transform, width: f64, height: f64) -> Rect {
    let scaled_width = width * transform.scale_x.abs();
    let scaled_height = height * transform.scale_y.abs();

    if transform.rotation == 0.0 {
        return Rect::from_origin_size(
            Point {
                x: transform.x,
                y: transform.y,
            },
            scaled_width,
            scaled_height,
        );
    }

    let radians = transform.rotation.to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();
    let rotated_width = scaled_width * cos + scaled_height * sin;
    let rotated_height = scaled_width * sin + scaled_height * cos;

    Rect::from_origin_size(
        Point {
            x: transform.x,
            y: transform.y,
        },
        rotated_width,
        rotated_height,
    )
}

fn expand_bounds_for_effects(node: &SceneNode, bounds: Rect) -> Rect {
    let mut expanded = bounds;

    if let Some(blur_radius) = node.style_blur_radius() {
        expanded = inflate_rect(expanded, blur_radius * 2.0);
    }

    if let Some(shadow) = node.style_shadow() {
        let shadow_bounds = Rect {
            x: bounds.x + shadow.offset_x,
            y: bounds.y + shadow.offset_y,
            width: bounds.width,
            height: bounds.height,
        };
        expanded = expanded.union(inflate_rect(shadow_bounds, shadow.blur_radius * 2.0));
    }

    expanded
}

fn inflate_rect(rect: Rect, amount: f64) -> Rect {
    Rect {
        x: rect.x - amount,
        y: rect.y - amount,
        width: rect.width + amount * 2.0,
        height: rect.height + amount * 2.0,
    }
}

fn estimate_text_bounds(transform: &Transform, text: &str, font_size: f64) -> Rect {
    let lines = text.lines().collect::<Vec<_>>();
    let max_line_chars = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let width = max_line_chars * font_size * 0.6 * transform.scale_x.abs();
    let height = lines.len() as f64 * font_size * 1.2 * transform.scale_y.abs();

    Rect::from_origin_size(
        Point {
            x: transform.x,
            y: transform.y,
        },
        width,
        height,
    )
}

fn bounds_from_points(transform: &Transform, points: &[PathPoint]) -> Option<Rect> {
    let mut transformed = points
        .iter()
        .map(|point| transform_point(transform, point.x, point.y));

    let first = transformed.next()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for point in transformed {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Some(Rect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    })
}

fn transform_point(transform: &Transform, x: f64, y: f64) -> Point {
    let scaled_x = x * transform.scale_x;
    let scaled_y = y * transform.scale_y;
    let radians = transform.rotation.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();

    Point {
        x: transform.x + scaled_x * cos - scaled_y * sin,
        y: transform.y + scaled_x * sin + scaled_y * cos,
    }
}

fn set_object_string(object: &mut JsonObject, key: &str, value: String) {
    object.insert(key.to_string(), Value::String(value));
}

fn hit_test_node(node: &SceneNode, point: Point, hits: &mut Vec<String>) {
    if !node.visible {
        return;
    }

    for child in node.children.iter().rev() {
        hit_test_node(child, point, hits);
    }

    if contains_point_for_node(node, point) {
        hits.push(node.id.clone());
    }
}

fn point_in_polygon(point: Point, polygon: &[Point]) -> bool {
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];

    for &current in polygon {
        let intersects = ((current.y > point.y) != (previous.y > point.y))
            && (point.x
                < (previous.x - current.x) * (point.y - current.y)
                    / ((previous.y - current.y) + f64::EPSILON)
                    + current.x);

        if intersects {
            inside = !inside;
        }

        previous = current;
    }

    inside
}

#[cfg(test)]
mod tests {
    use scene_schema::{NodeType, Transform, parse_scene_str};

    use super::{
        ComponentRegistry, DocumentCommand, Point, RuntimeDocument, bounds_for_node,
        contains_point_for_node, find_node, validate_registry_compatibility,
    };

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
    const SHAPES_STUDY: &str = include_str!("../../../examples/shapes_study.vsd.json");

    fn make_runtime() -> RuntimeDocument {
        let scene = parse_scene_str(BASIC_POSTER).expect("example scene should parse");
        RuntimeDocument::new(scene, ComponentRegistry::mvp()).expect("runtime should be valid")
    }

    #[test]
    fn registry_accepts_example_document() {
        let scene = parse_scene_str(BASIC_POSTER).expect("example scene should parse");
        let issues = validate_registry_compatibility(&scene, &ComponentRegistry::mvp());
        assert!(issues.is_empty(), "expected no issues, found {issues:?}");
    }

    #[test]
    fn visits_scene_depth_first() {
        let runtime = make_runtime();
        let mut visited = Vec::new();

        runtime.visit_depth_first(|visit| {
            visited.push((visit.depth, visit.node.id.clone()));
        });

        assert_eq!(
            visited,
            vec![
                (0, "root".to_string()),
                (1, "bg_rect".to_string()),
                (1, "headline".to_string())
            ]
        );
    }

    #[test]
    fn renames_node_with_command() {
        let mut runtime = make_runtime();
        runtime
            .apply(DocumentCommand::RenameNode {
                node_id: "headline".to_string(),
                new_name: "Title".to_string(),
            })
            .expect("rename should succeed");

        let node = runtime.find_node("headline").expect("node should exist");
        assert_eq!(node.name, "Title");
    }

    #[test]
    fn updates_transform_with_command() {
        let mut runtime = make_runtime();
        runtime
            .apply(DocumentCommand::SetNodeTransform {
                node_id: "headline".to_string(),
                transform: Transform {
                    x: 240.0,
                    y: 260.0,
                    scale_x: 1.0,
                    scale_y: 1.0,
                    rotation: 5.0,
                    opacity: 0.8,
                },
            })
            .expect("transform update should succeed");

        let node = runtime.find_node("headline").expect("node should exist");
        assert_eq!(node.transform.x, 240.0);
        assert_eq!(node.transform.opacity, 0.8);
    }

    #[test]
    fn inserts_child_under_group() {
        let mut runtime = make_runtime();
        let new_child = scene_schema::SceneNode {
            id: "new_rect".to_string(),
            node_type: NodeType::Rectangle,
            name: "New Rect".to_string(),
            visible: true,
            locked: false,
            blend_mode: scene_schema::BlendMode::Normal,
            transform: Transform {
                x: 10.0,
                y: 10.0,
                scale_x: 1.0,
                scale_y: 1.0,
                rotation: 0.0,
                opacity: 1.0,
            },
            params: Default::default(),
            style: Default::default(),
            children: Vec::new(),
            meta: Default::default(),
        };

        runtime
            .apply(DocumentCommand::InsertChild {
                parent_id: "root".to_string(),
                child: new_child,
                index: None,
            })
            .expect("insert should succeed");

        assert!(find_node(&runtime.scene().document.root, "new_rect").is_some());
    }

    #[test]
    fn rejects_insert_into_leaf_node() {
        let mut runtime = make_runtime();
        let new_child = runtime
            .find_node("bg_rect")
            .expect("node should exist")
            .clone();

        let error = runtime
            .apply(DocumentCommand::InsertChild {
                parent_id: "headline".to_string(),
                child: new_child,
                index: None,
            })
            .expect_err("insert should fail");

        assert!(matches!(
            error,
            super::CommandError::InvalidChildParent { .. }
        ));
    }

    #[test]
    fn removes_node_from_scene() {
        let mut runtime = make_runtime();
        runtime
            .apply(DocumentCommand::RemoveNode {
                node_id: "bg_rect".to_string(),
            })
            .expect("remove should succeed");

        assert!(runtime.find_node("bg_rect").is_none());
    }

    #[test]
    fn computes_bounds_for_rectangle_and_group() {
        let runtime = make_runtime();

        let rect_bounds = runtime
            .node_bounds("bg_rect")
            .expect("rectangle bounds should exist");
        assert!(rect_bounds.x < 120.0);
        assert!(rect_bounds.y < 100.0);
        assert!(rect_bounds.width > 1360.0);
        assert!(rect_bounds.height > 700.0);

        let group_bounds = runtime
            .node_bounds("root")
            .expect("group bounds should exist");
        assert!(group_bounds.width >= rect_bounds.width);
        assert!(group_bounds.height >= rect_bounds.height);
    }

    #[test]
    fn hit_test_returns_topmost_child_first() {
        let runtime = make_runtime();
        let hits = runtime.hit_test(Point { x: 250.0, y: 250.0 });

        assert_eq!(hits.first().map(String::as_str), Some("headline"));
        assert!(hits.iter().any(|id| id == "bg_rect"));
        assert!(hits.iter().any(|id| id == "root"));
    }

    #[test]
    fn bounds_helper_returns_none_for_path_without_geometry() {
        let path_node = scene_schema::SceneNode {
            id: "path".to_string(),
            node_type: NodeType::Path,
            name: "Path".to_string(),
            visible: true,
            locked: false,
            blend_mode: scene_schema::BlendMode::Normal,
            transform: Transform {
                x: 0.0,
                y: 0.0,
                scale_x: 1.0,
                scale_y: 1.0,
                rotation: 0.0,
                opacity: 1.0,
            },
            params: Default::default(),
            style: Default::default(),
            children: Vec::new(),
            meta: Default::default(),
        };

        assert!(bounds_for_node(&path_node).is_none());
    }

    #[test]
    fn computes_bounds_for_path_with_geometry() {
        let shapes = parse_scene_str(SHAPES_STUDY).expect("shapes scene should parse");
        let bounds =
            bounds_for_node(&shapes.document.root.children[2]).expect("path bounds should exist");

        assert!(bounds.width > 0.0);
        assert!(bounds.height > 0.0);
    }

    #[test]
    fn path_hit_testing_uses_polygon_not_just_bounds() {
        let shapes = parse_scene_str(SHAPES_STUDY).expect("shapes scene should parse");
        let diamond = &shapes.document.root.children[2];
        let bounds = bounds_for_node(diamond).expect("path bounds should exist");

        assert!(contains_point_for_node(
            diamond,
            Point {
                x: diamond.transform.x + 120.0,
                y: diamond.transform.y + 120.0,
            }
        ));

        assert!(!contains_point_for_node(
            diamond,
            Point {
                x: bounds.x + 2.0,
                y: bounds.y + 2.0,
            }
        ));
    }

    #[test]
    fn expands_bounds_for_shadow_and_blur_styles() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let rect_bounds =
            bounds_for_node(&scene.document.root.children[0]).expect("rect bounds should exist");
        let text_bounds =
            bounds_for_node(&scene.document.root.children[1]).expect("text bounds should exist");

        assert!(rect_bounds.x < 120.0);
        assert!(text_bounds.width > 0.0);
        assert!(text_bounds.height > 0.0);
    }
}
