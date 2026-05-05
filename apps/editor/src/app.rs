use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use renderer::{RenderItem, RenderKind, RenderPlan};
use scene_runtime::{ComponentRegistry, DocumentCommand, Point, Rect, RuntimeDocument};
use scene_schema::{JsonObject, SceneFile, parse_scene_str};
use serde::Serialize;

pub struct EditorApp {
    pub state: EditorState,
}

impl EditorApp {
    pub fn open_path(path: impl AsRef<Path>) -> Result<Self, EditorError> {
        let path = path.as_ref().to_path_buf();
        let source = fs::read_to_string(&path).map_err(|error| EditorError::ReadFailed {
            path: path.clone(),
            error,
        })?;
        let scene = parse_scene_str(&source).map_err(|error| EditorError::ParseFailed {
            path: path.clone(),
            error,
        })?;

        Self::from_scene(path, scene)
    }

    pub fn from_scene(path: PathBuf, scene: SceneFile) -> Result<Self, EditorError> {
        let runtime = RuntimeDocument::new(scene, ComponentRegistry::mvp())
            .map_err(EditorError::InvalidScene)?;
        let render_plan = renderer::build_render_plan(runtime.scene());
        let hierarchy = build_hierarchy(runtime.scene());
        let selected_node_id = hierarchy.first().map(|entry| entry.node_id.clone());

        Ok(Self {
            state: EditorState {
                document_path: path,
                runtime,
                render_plan,
                hierarchy,
                selected_node_id,
            },
        })
    }

    pub fn summary(&self) -> EditorSummary {
        let selected = self
            .state
            .selected_node_id
            .as_ref()
            .and_then(|id| self.state.runtime.find_node(id))
            .map(|node| SelectedNodeSummary {
                id: node.id.clone(),
                node_type: format!("{:?}", node.node_type),
                name: node.name.clone(),
            });

        EditorSummary {
            document_path: self.state.document_path.clone(),
            document_name: self.state.runtime.scene().document.name.clone(),
            canvas_width: self.state.runtime.scene().document.width,
            canvas_height: self.state.runtime.scene().document.height,
            render_item_count: self.state.render_plan.items.len(),
            selected,
        }
    }

    pub fn export_png(&self, output_path: impl AsRef<Path>) -> Result<(), EditorError> {
        #[cfg(feature = "skia-safe-backend")]
        {
            let document = &self.state.runtime.scene().document;
            renderer::skia_backend::write_plan_png(
                &self.state.render_plan,
                document.width.round() as u32,
                document.height.round() as u32,
                output_path,
            )
            .map_err(EditorError::ExportFailed)
        }

        #[cfg(not(feature = "skia-safe-backend"))]
        {
            let _ = output_path;
            Err(EditorError::ExportUnavailable)
        }
    }

    pub fn rename_node(
        &mut self,
        node_id: &str,
        new_name: impl Into<String>,
    ) -> Result<(), EditorError> {
        let new_name = new_name.into();
        self.state
            .runtime
            .apply(DocumentCommand::RenameNode {
                node_id: node_id.to_string(),
                new_name,
            })
            .map_err(EditorError::CommandFailed)?;
        self.refresh_derived_state();
        self.state.selected_node_id = Some(node_id.to_string());
        Ok(())
    }

    pub fn set_position(&mut self, node_id: &str, x: f64, y: f64) -> Result<(), EditorError> {
        self.state
            .runtime
            .apply(DocumentCommand::SetNodePosition {
                node_id: node_id.to_string(),
                x,
                y,
            })
            .map_err(EditorError::CommandFailed)?;
        self.refresh_derived_state();
        self.state.selected_node_id = Some(node_id.to_string());
        Ok(())
    }

    pub fn replace_node_params(
        &mut self,
        node_id: &str,
        params: JsonObject,
    ) -> Result<(), EditorError> {
        self.state
            .runtime
            .apply(DocumentCommand::SetNodeParamsObject {
                node_id: node_id.to_string(),
                params,
            })
            .map_err(EditorError::CommandFailed)?;
        self.refresh_derived_state();
        self.state.selected_node_id = Some(node_id.to_string());
        Ok(())
    }

    pub fn replace_node_style(
        &mut self,
        node_id: &str,
        style: JsonObject,
    ) -> Result<(), EditorError> {
        self.state
            .runtime
            .apply(DocumentCommand::SetNodeStyleObject {
                node_id: node_id.to_string(),
                style,
            })
            .map_err(EditorError::CommandFailed)?;
        self.refresh_derived_state();
        self.state.selected_node_id = Some(node_id.to_string());
        Ok(())
    }

    pub fn save_to_path(&self, output_path: impl AsRef<Path>) -> Result<(), EditorError> {
        let output_path = output_path.as_ref();
        let serialized = serde_json::to_string_pretty(self.state.runtime.scene())
            .map_err(EditorError::SerializeFailed)?;
        fs::write(output_path, format!("{serialized}\n")).map_err(|error| {
            EditorError::WriteFailed {
                path: output_path.to_path_buf(),
                error,
            }
        })
    }

    pub fn view_model(&self) -> EditorViewModel {
        let hierarchy_by_id = self
            .state
            .hierarchy
            .iter()
            .map(|entry| (entry.node_id.as_str(), entry))
            .collect::<std::collections::HashMap<_, _>>();
        let selected_node_id = self.state.selected_node_id.clone();
        let nodes = self
            .state
            .hierarchy
            .iter()
            .filter_map(|entry| self.state.runtime.find_node(&entry.node_id))
            .map(|node| EditorNodeViewModel {
                depth: hierarchy_by_id
                    .get(node.id.as_str())
                    .map(|entry| entry.depth)
                    .unwrap_or(0),
                id: node.id.clone(),
                node_type: format!("{:?}", node.node_type),
                name: node.name.clone(),
                position_x: node.transform.x,
                position_y: node.transform.y,
                params: serde_json::Value::Object(node.params.clone()),
                style: serde_json::Value::Object(node.style.clone()),
                bounds: self
                    .state
                    .runtime
                    .node_bounds(&node.id)
                    .map(EditorRectViewModel::from),
            })
            .collect();
        let render_items = self
            .state
            .render_plan
            .items
            .iter()
            .map(EditorCanvasItemViewModel::from_render_item)
            .collect();

        EditorViewModel {
            document_path: self.state.document_path.to_string_lossy().to_string(),
            document_name: self.state.runtime.scene().document.name.clone(),
            canvas_width: self.state.runtime.scene().document.width,
            canvas_height: self.state.runtime.scene().document.height,
            background: self.state.runtime.scene().document.background.color.clone(),
            render_item_count: self.state.render_plan.items.len(),
            selected_node_id,
            nodes,
            render_items,
        }
    }

    fn refresh_derived_state(&mut self) {
        self.state.render_plan = renderer::build_render_plan(self.state.runtime.scene());
        self.state.hierarchy = build_hierarchy(self.state.runtime.scene());

        if let Some(selected_node_id) = &self.state.selected_node_id
            && self.state.runtime.find_node(selected_node_id).is_none()
        {
            self.state.selected_node_id = self
                .state
                .hierarchy
                .first()
                .map(|entry| entry.node_id.clone());
        }
    }
}

pub struct EditorState {
    pub document_path: PathBuf,
    pub runtime: RuntimeDocument,
    pub render_plan: RenderPlan,
    pub hierarchy: Vec<HierarchyEntry>,
    pub selected_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyEntry {
    pub depth: usize,
    pub node_id: String,
    pub name: String,
    pub node_type: String,
}

pub struct EditorSummary {
    pub document_path: PathBuf,
    pub document_name: String,
    pub canvas_width: f64,
    pub canvas_height: f64,
    pub render_item_count: usize,
    pub selected: Option<SelectedNodeSummary>,
}

pub struct SelectedNodeSummary {
    pub id: String,
    pub node_type: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditorViewModel {
    pub document_path: String,
    pub document_name: String,
    pub canvas_width: f64,
    pub canvas_height: f64,
    pub background: String,
    pub render_item_count: usize,
    pub selected_node_id: Option<String>,
    pub nodes: Vec<EditorNodeViewModel>,
    pub render_items: Vec<EditorCanvasItemViewModel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditorNodeViewModel {
    pub depth: usize,
    pub id: String,
    pub node_type: String,
    pub name: String,
    pub position_x: f64,
    pub position_y: f64,
    pub params: serde_json::Value,
    pub style: serde_json::Value,
    pub bounds: Option<EditorRectViewModel>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditorCanvasItemViewModel {
    pub node_id: String,
    pub kind: String,
    pub opacity: f64,
    pub blend_mode: String,
    pub bounds: Option<EditorRectViewModel>,
    pub fill: Option<String>,
    pub corner_radius: Option<f64>,
    pub origin: Option<EditorPointViewModel>,
    pub points: Vec<EditorPointViewModel>,
    pub closed: Option<bool>,
    pub text: Option<String>,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub image_ref: Option<String>,
    pub blur_radius: Option<f64>,
    pub shadow: Option<EditorShadowViewModel>,
}

impl EditorCanvasItemViewModel {
    fn from_render_item(item: &RenderItem) -> Self {
        let (
            kind,
            fill,
            corner_radius,
            origin,
            points,
            closed,
            text,
            font_size,
            font_family,
            image_ref,
        ) = match &item.kind {
            RenderKind::Rectangle(rectangle) => (
                "Rectangle".to_string(),
                rectangle.fill.clone(),
                Some(rectangle.corner_radius),
                None,
                Vec::new(),
                None,
                None,
                None,
                None,
                None,
            ),
            RenderKind::Ellipse(ellipse) => (
                "Ellipse".to_string(),
                ellipse.fill.clone(),
                None,
                None,
                Vec::new(),
                None,
                None,
                None,
                None,
                None,
            ),
            RenderKind::Path(path) => (
                "Path".to_string(),
                path.fill.clone(),
                None,
                None,
                path.points
                    .iter()
                    .copied()
                    .map(EditorPointViewModel::from)
                    .collect(),
                Some(path.closed),
                None,
                None,
                None,
                None,
            ),
            RenderKind::Text(text_item) => (
                "Text".to_string(),
                text_item.fill.clone(),
                None,
                Some(EditorPointViewModel::from(text_item.origin)),
                Vec::new(),
                None,
                Some(text_item.text.clone()),
                Some(text_item.font_size),
                text_item.font_family.clone(),
                None,
            ),
            RenderKind::ImageLayer(image) => (
                "ImageLayer".to_string(),
                None,
                None,
                None,
                Vec::new(),
                None,
                None,
                None,
                None,
                image.image_ref.clone(),
            ),
        };

        Self {
            node_id: item.node_id.clone(),
            kind,
            opacity: item.opacity,
            blend_mode: format!("{:?}", item.blend_mode),
            bounds: item.bounds.map(EditorRectViewModel::from),
            fill,
            corner_radius,
            origin,
            points,
            closed,
            text,
            font_size,
            font_family,
            image_ref,
            blur_radius: item.effects.blur_radius,
            shadow: item
                .effects
                .shadow
                .as_ref()
                .map(EditorShadowViewModel::from),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct EditorRectViewModel {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl From<Rect> for EditorRectViewModel {
    fn from(value: Rect) -> Self {
        Self {
            x: value.x,
            y: value.y,
            width: value.width,
            height: value.height,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct EditorPointViewModel {
    pub x: f64,
    pub y: f64,
}

impl From<Point> for EditorPointViewModel {
    fn from(value: Point) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EditorShadowViewModel {
    pub color: String,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur_radius: f64,
}

impl From<&renderer::RenderShadow> for EditorShadowViewModel {
    fn from(value: &renderer::RenderShadow) -> Self {
        Self {
            color: value.color.clone(),
            offset_x: value.offset_x,
            offset_y: value.offset_y,
            blur_radius: value.blur_radius,
        }
    }
}

#[derive(Debug)]
pub enum EditorError {
    ReadFailed {
        path: PathBuf,
        error: std::io::Error,
    },
    ParseFailed {
        path: PathBuf,
        error: serde_json::Error,
    },
    InvalidScene(Vec<scene_runtime::RuntimeIssue>),
    CommandFailed(scene_runtime::CommandError),
    SerializeFailed(serde_json::Error),
    WriteFailed {
        path: PathBuf,
        error: std::io::Error,
    },
    #[cfg(not(feature = "skia-safe-backend"))]
    ExportUnavailable,
    ExportFailed(renderer::skia_backend::SkiaRenderError),
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadFailed { path, error } => {
                write!(f, "failed to read scene at {}: {error}", path.display())
            }
            Self::ParseFailed { path, error } => {
                write!(f, "failed to parse scene at {}: {error}", path.display())
            }
            Self::InvalidScene(issues) => {
                write!(f, "scene validation failed with {} issue(s)", issues.len())
            }
            Self::CommandFailed(error) => write!(f, "scene edit failed: {error:?}"),
            Self::SerializeFailed(error) => write!(f, "failed to serialize scene: {error}"),
            Self::WriteFailed { path, error } => {
                write!(f, "failed to write scene at {}: {error}", path.display())
            }
            #[cfg(not(feature = "skia-safe-backend"))]
            Self::ExportUnavailable => {
                write!(f, "PNG export requires the skia-safe-backend feature")
            }
            Self::ExportFailed(error) => write!(f, "PNG export failed: {error}"),
        }
    }
}

impl std::error::Error for EditorError {}

fn build_hierarchy(scene: &SceneFile) -> Vec<HierarchyEntry> {
    let mut entries = Vec::new();
    visit_hierarchy(&scene.document.root, 0, &mut entries);
    entries
}

fn visit_hierarchy(
    node: &scene_schema::SceneNode,
    depth: usize,
    entries: &mut Vec<HierarchyEntry>,
) {
    entries.push(HierarchyEntry {
        depth,
        node_id: node.id.clone(),
        name: node.name.clone(),
        node_type: format!("{:?}", node.node_type),
    });

    for child in &node.children {
        visit_hierarchy(child, depth + 1, entries);
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::EditorApp;
    use scene_schema::parse_scene_str;

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");

    #[test]
    fn builds_editor_state_from_scene() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");

        assert_eq!(app.state.hierarchy.len(), 3);
        assert_eq!(app.state.render_plan.items.len(), 2);
        assert_eq!(app.state.selected_node_id.as_deref(), Some("root"));
    }

    #[test]
    fn builds_view_model_with_render_items() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");
        let view_model = app.view_model();

        assert_eq!(view_model.document_name, "Basic Poster");
        assert_eq!(view_model.nodes.len(), 3);
        assert_eq!(view_model.render_items.len(), 2);
        assert_eq!(view_model.render_items[0].kind, "Rectangle");
        assert_eq!(view_model.render_items[1].kind, "Text");
    }

    #[test]
    fn exposes_summary() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");
        let summary = app.summary();

        assert_eq!(
            summary.document_path,
            PathBuf::from("basic_poster.vsd.json")
        );
        assert_eq!(summary.document_name, "Basic Poster");
        assert_eq!(summary.render_item_count, 2);
        assert!(summary.selected.is_some());
    }

    #[test]
    fn exposes_view_model() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");
        let view_model = app.view_model();

        assert_eq!(view_model.document_name, "Basic Poster");
        assert_eq!(view_model.nodes.len(), 3);
        assert_eq!(view_model.selected_node_id.as_deref(), Some("root"));
    }

    #[test]
    fn rename_node_updates_summary_and_view_model() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let mut app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");

        app.rename_node("headline", "Title Block")
            .expect("rename should succeed");

        let renamed = app
            .state
            .runtime
            .find_node("headline")
            .expect("headline should exist");
        assert_eq!(renamed.name, "Title Block");
        assert_eq!(
            app.view_model().selected_node_id.as_deref(),
            Some("headline")
        );
    }

    #[test]
    fn replaces_params_style_and_position() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let mut app = EditorApp::from_scene(PathBuf::from("basic_poster.vsd.json"), scene)
            .expect("editor app should initialize");
        let params = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
            r#"{"text":"JSON MODE","fontFamily":"Inter","fontSize":72,"lineHeight":1.0}"#,
        )
        .expect("params json should parse");
        let style = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(
            r##"{"fill":"#445566"}"##,
        )
        .expect("style json should parse");

        app.set_position("headline", 320.0, 360.0)
            .expect("position update should succeed");
        app.replace_node_params("headline", params)
            .expect("params replace should succeed");
        app.replace_node_style("headline", style)
            .expect("style replace should succeed");

        let headline = app
            .state
            .runtime
            .find_node("headline")
            .expect("headline should exist");
        assert_eq!(headline.transform.x, 320.0);
        assert_eq!(headline.transform.y, 360.0);
        assert_eq!(
            headline
                .text_params()
                .expect("text params should exist")
                .text,
            "JSON MODE"
        );
        assert_eq!(headline.style_fill().as_deref(), Some("#445566"));
    }
}
