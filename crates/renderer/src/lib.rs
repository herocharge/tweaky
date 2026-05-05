use scene_schema::{JsonObject, SceneFile, SceneNode, Transform};
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
}

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

    let bounds = match &kind {
        RenderKind::Rectangle(primitive) => Some(primitive.bounds),
        RenderKind::Ellipse(primitive) => Some(primitive.bounds),
        RenderKind::Path(primitive) => primitive.bounds,
        RenderKind::Text(primitive) => Some(estimate_text_bounds(primitive, &node.transform)),
        RenderKind::ImageLayer(primitive) => Some(primitive.bounds),
    };

    Some(RenderItem {
        node_id: node.id.clone(),
        kind,
        opacity: node.transform.opacity,
        blend_mode: node.blend_mode,
        bounds,
    })
}

fn rectangle_primitive(node: &SceneNode) -> Option<RectanglePrimitive> {
    let width = get_number(&node.params, "width")?;
    let height = get_number(&node.params, "height")?;
    let corner_radius = get_number(&node.params, "cornerRadius").unwrap_or(0.0);
    let bounds = transformed_rect(&node.transform, width, height);

    Some(RectanglePrimitive {
        bounds,
        corner_radius,
        fill: get_string(&node.style, "fill"),
    })
}

fn ellipse_primitive(node: &SceneNode) -> Option<EllipsePrimitive> {
    let radius_x = get_number(&node.params, "radiusX")?;
    let radius_y = get_number(&node.params, "radiusY")?;
    let bounds = transformed_rect(&node.transform, radius_x * 2.0, radius_y * 2.0);

    Some(EllipsePrimitive {
        bounds,
        fill: get_string(&node.style, "fill"),
    })
}

fn path_primitive(node: &SceneNode) -> PathPrimitive {
    PathPrimitive {
        bounds: None,
        fill: get_string(&node.style, "fill"),
    }
}

fn text_primitive(node: &SceneNode) -> Option<TextPrimitive> {
    let text = get_string(&node.params, "text")?;
    let font_size = get_number(&node.params, "fontSize").unwrap_or(16.0);

    Some(TextPrimitive {
        origin: Point {
            x: node.transform.x,
            y: node.transform.y,
        },
        text,
        font_size,
        font_family: get_string(&node.params, "fontFamily"),
        fill: get_string(&node.style, "fill"),
    })
}

fn image_layer_primitive(node: &SceneNode) -> Option<ImageLayerPrimitive> {
    let width = get_number(&node.params, "displayWidth")?;
    let height = get_number(&node.params, "displayHeight")?;

    Some(ImageLayerPrimitive {
        bounds: transformed_rect(&node.transform, width, height),
        image_ref: get_string(&node.params, "imageRef"),
    })
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

fn estimate_text_bounds(primitive: &TextPrimitive, transform: &Transform) -> Rect {
    let lines = primitive.text.lines().collect::<Vec<_>>();
    let max_line_chars = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as f64;
    let width = max_line_chars * primitive.font_size * 0.6 * transform.scale_x.abs();
    let height = lines.len() as f64 * primitive.font_size * 1.2 * transform.scale_y.abs();

    Rect::from_origin_size(primitive.origin, width, height)
}

fn get_number(object: &JsonObject, key: &str) -> Option<f64> {
    object.get(key)?.as_f64()
}

fn get_string(object: &JsonObject, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use scene_schema::parse_scene_str;

    use super::{RenderBackend, RenderItem, build_render_plan, render_with_backend};

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");
    const HYBRID_SCENE: &str = include_str!("../../../examples/hybrid_scene.vsd.json");

    #[test]
    fn builds_render_plan_for_basic_poster() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let plan = build_render_plan(&scene);

        assert_eq!(plan.background.color, "#f5f1e8");
        assert_eq!(plan.items.len(), 2);
        assert_eq!(plan.items[0].node_id, "bg_rect");
        assert_eq!(plan.items[1].node_id, "headline");
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
