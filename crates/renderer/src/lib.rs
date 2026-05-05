use scene_runtime::bounds_for_node;
pub use scene_runtime::{Point, Rect};
use scene_schema::{SceneFile, SceneNode};

#[cfg(feature = "skia-safe-backend")]
pub mod skia_backend;

#[derive(Debug, Clone, PartialEq)]
pub struct RenderPlan {
    pub background: RenderBackground,
    pub items: Vec<RenderItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderBackground {
    pub color: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderItem {
    pub node_id: String,
    pub kind: RenderKind,
    pub opacity: f64,
    pub blend_mode: scene_schema::BlendMode,
    pub bounds: Option<Rect>,
    pub effects: RenderEffects,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RenderEffects {
    pub blur_radius: Option<f64>,
    pub shadow: Option<RenderShadow>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderShadow {
    pub color: String,
    pub offset_x: f64,
    pub offset_y: f64,
    pub blur_radius: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderKind {
    Rectangle(RectanglePrimitive),
    Ellipse(EllipsePrimitive),
    Path(PathPrimitive),
    Text(TextPrimitive),
    ImageLayer(ImageLayerPrimitive),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RectanglePrimitive {
    pub bounds: Rect,
    pub corner_radius: f64,
    pub fill: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EllipsePrimitive {
    pub bounds: Rect,
    pub fill: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PathPrimitive {
    pub bounds: Option<Rect>,
    pub points: Vec<Point>,
    pub closed: bool,
    pub fill: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextPrimitive {
    pub origin: Point,
    pub text: String,
    pub font_size: f64,
    pub font_family: Option<String>,
    pub fill: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageLayerPrimitive {
    pub bounds: Rect,
    pub image_ref: Option<String>,
}

pub trait RenderBackend {
    type Error;

    fn clear(&mut self, background: &RenderBackground) -> Result<(), Self::Error>;
    fn draw_item(&mut self, item: &RenderItem) -> Result<(), Self::Error>;
}

pub fn build_render_plan(scene: &SceneFile) -> RenderPlan {
    let mut items = Vec::new();
    collect_render_items(&scene.document.root, &mut items);

    RenderPlan {
        background: RenderBackground {
            color: scene.document.background.color.clone(),
        },
        items,
    }
}

pub fn render_with_backend<B: RenderBackend>(
    backend: &mut B,
    plan: &RenderPlan,
) -> Result<(), B::Error> {
    backend.clear(&plan.background)?;
    for item in &plan.items {
        backend.draw_item(item)?;
    }
    Ok(())
}

fn collect_render_items(node: &SceneNode, items: &mut Vec<RenderItem>) {
    if !node.visible {
        return;
    }

    if let Some(item) = node_to_render_item(node) {
        items.push(item);
    }

    for child in &node.children {
        collect_render_items(child, items);
    }
}

fn node_to_render_item(node: &SceneNode) -> Option<RenderItem> {
    let kind = match node.node_type {
        scene_schema::NodeType::Group
        | scene_schema::NodeType::Shadow
        | scene_schema::NodeType::Blur => return None,
        scene_schema::NodeType::Rectangle => RenderKind::Rectangle(rectangle_primitive(node)?),
        scene_schema::NodeType::Ellipse => RenderKind::Ellipse(ellipse_primitive(node)?),
        scene_schema::NodeType::Path => RenderKind::Path(path_primitive(node)),
        scene_schema::NodeType::Text => RenderKind::Text(text_primitive(node)?),
        scene_schema::NodeType::ImageLayer => RenderKind::ImageLayer(image_layer_primitive(node)?),
    };

    let bounds = bounds_for_node(node);

    Some(RenderItem {
        node_id: node.id.clone(),
        kind,
        opacity: node.transform.opacity,
        blend_mode: node.blend_mode,
        bounds,
        effects: RenderEffects {
            blur_radius: node.style_blur_radius(),
            shadow: node.style_shadow().map(|shadow| RenderShadow {
                color: shadow.color,
                offset_x: shadow.offset_x,
                offset_y: shadow.offset_y,
                blur_radius: shadow.blur_radius,
            }),
        },
    })
}

fn rectangle_primitive(node: &SceneNode) -> Option<RectanglePrimitive> {
    let params = node.rectangle_params()?;
    let bounds = bounds_for_node(node)?;

    Some(RectanglePrimitive {
        bounds,
        corner_radius: params.corner_radius,
        fill: node.style_fill(),
    })
}

fn ellipse_primitive(node: &SceneNode) -> Option<EllipsePrimitive> {
    let bounds = bounds_for_node(node)?;

    Some(EllipsePrimitive {
        bounds,
        fill: node.style_fill(),
    })
}

fn path_primitive(node: &SceneNode) -> PathPrimitive {
    let params = node.path_params();
    PathPrimitive {
        bounds: bounds_for_node(node),
        points: params
            .as_ref()
            .map(|params| {
                params
                    .points
                    .iter()
                    .map(|point| transform_path_point(node, point.x, point.y))
                    .collect()
            })
            .unwrap_or_default(),
        closed: params.as_ref().map(|params| params.closed).unwrap_or(true),
        fill: node.style_fill(),
    }
}

fn transform_path_point(node: &SceneNode, x: f64, y: f64) -> Point {
    let scaled_x = x * node.transform.scale_x;
    let scaled_y = y * node.transform.scale_y;
    let radians = node.transform.rotation.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();

    Point {
        x: node.transform.x + scaled_x * cos - scaled_y * sin,
        y: node.transform.y + scaled_x * sin + scaled_y * cos,
    }
}

fn text_primitive(node: &SceneNode) -> Option<TextPrimitive> {
    let params = node.text_params()?;

    Some(TextPrimitive {
        origin: Point {
            x: node.transform.x,
            y: node.transform.y,
        },
        text: params.text,
        font_size: params.font_size,
        font_family: params.font_family,
        fill: node.style_fill(),
    })
}

fn image_layer_primitive(node: &SceneNode) -> Option<ImageLayerPrimitive> {
    let params = node.image_layer_params()?;

    Some(ImageLayerPrimitive {
        bounds: bounds_for_node(node)?,
        image_ref: Some(params.image_ref),
    })
}

#[cfg(test)]
mod tests {
    use scene_runtime::bounds_for_node;
    use scene_schema::parse_scene_str;

    use super::{RenderBackend, RenderItem, build_render_plan, render_with_backend};

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
    const HYBRID_SCENE: &str = include_str!("../../../examples/hybrid_scene.vsd.json");
    const SHAPES_STUDY: &str = include_str!("../../../examples/shapes_study.vsd.json");

    #[test]
    fn builds_render_plan_for_basic_poster() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let plan = build_render_plan(&scene);

        assert_eq!(plan.background.color, "#f5f1e8");
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items[0].node_id, "bg_rect");
        assert_eq!(plan.items[1].node_id, "headline");
        assert_eq!(plan.items[0].effects.blur_radius, Some(6.0));
        assert!(plan.items[1].effects.shadow.is_some());
    }

    #[test]
    fn builds_image_layer_render_item() {
        let scene = parse_scene_str(HYBRID_SCENE).expect("scene should parse");
        let plan = build_render_plan(&scene);

        assert!(plan.items.iter().any(|item| item.node_id == "paint_layer"));
    }

    #[test]
    fn backend_can_consume_render_plan() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let plan = build_render_plan(&scene);
        let mut backend = RecordingBackend::default();

        render_with_backend(&mut backend, &plan).expect("render should succeed");

        assert_eq!(backend.backgrounds, vec!["#f5f1e8".to_string()]);
        assert_eq!(backend.items.len(), 2);
    }

    #[test]
    fn builds_path_render_item() {
        let scene = parse_scene_str(SHAPES_STUDY).expect("scene should parse");
        let plan = build_render_plan(&scene);
        let path_item = plan
            .items
            .iter()
            .find(|item| item.node_id == "diamond")
            .expect("path item should exist");

        match &path_item.kind {
            super::RenderKind::Path(path) => {
                assert_eq!(path.points.len(), 4);
                assert!(path.closed);
                assert!(path.bounds.is_some());
            }
            other => panic!("expected path render kind, found {other:?}"),
        }
    }

    #[test]
    fn render_plan_uses_runtime_bounds() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let runtime_bounds =
            bounds_for_node(&scene.document.root.children[0]).expect("runtime bounds should exist");
        let plan = build_render_plan(&scene);
        let item_bounds = plan.items[0].bounds.expect("item bounds should exist");

        assert_eq!(item_bounds, runtime_bounds);
    }

    #[derive(Default)]
    struct RecordingBackend {
        backgrounds: Vec<String>,
        items: Vec<String>,
    }

    impl RenderBackend for RecordingBackend {
        type Error = ();

        fn clear(&mut self, background: &super::RenderBackground) -> Result<(), Self::Error> {
            self.backgrounds.push(background.color.clone());
            Ok(())
        }

        fn draw_item(&mut self, item: &RenderItem) -> Result<(), Self::Error> {
            self.items.push(item.node_id.clone());
            Ok(())
        }
    }
}
