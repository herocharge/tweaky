use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use renderer::RenderPlan;
use scene_runtime::{ComponentRegistry, RuntimeDocument};
use scene_schema::{SceneFile, parse_scene_str};

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
}
