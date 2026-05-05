use std::fmt;
use std::path::Path;

use skia_safe::{
    BlurStyle, Color, EncodedImageFormat, Font, FontMgr, FontStyle, MaskFilter, Paint,
    PathBuilder, RRect, Rect as SkRect, Surface, surfaces,
};

use crate::{
    EllipsePrimitive, PathPrimitive, RenderBackground, RenderItem, RenderKind, RenderPlan,
    RenderShadow, TextPrimitive,
};

#[derive(Debug)]
pub enum SkiaRenderError {
    SurfaceCreationFailed { width: i32, height: i32 },
    EncodeFailed,
    UnsupportedDimension { width: u32, height: u32 },
    Io(std::io::Error),
}

impl fmt::Display for SkiaRenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SurfaceCreationFailed { width, height } => {
                write!(f, "failed to create Skia raster surface {width}x{height}")
            }
            Self::EncodeFailed => write!(f, "failed to encode rendered image as PNG"),
            Self::UnsupportedDimension { width, height } => {
                write!(f, "unsupported surface dimensions {width}x{height}")
            }
            Self::Io(error) => write!(f, "failed to write rendered file: {error}"),
        }
    }
}

impl std::error::Error for SkiaRenderError {}

pub fn render_plan_to_png(
    plan: &RenderPlan,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, SkiaRenderError> {
    let width = i32::try_from(width)
        .map_err(|_| SkiaRenderError::UnsupportedDimension { width, height })?;
    let height = i32::try_from(height).map_err(|_| SkiaRenderError::UnsupportedDimension {
        width: width as u32,
        height,
    })?;

    let mut surface = surfaces::raster_n32_premul((width, height))
        .ok_or(SkiaRenderError::SurfaceCreationFailed { width, height })?;

    render_plan_to_surface(plan, &mut surface);

    let image = surface.image_snapshot();
    let data = image
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or(SkiaRenderError::EncodeFailed)?;

    Ok(data.as_bytes().to_vec())
}

pub fn write_plan_png(
    plan: &RenderPlan,
    width: u32,
    height: u32,
    path: impl AsRef<Path>,
) -> Result<(), SkiaRenderError> {
    let png = render_plan_to_png(plan, width, height)?;
    std::fs::write(path, png).map_err(SkiaRenderError::Io)?;
    Ok(())
}

pub fn render_plan_to_surface(plan: &RenderPlan, surface: &mut Surface) {
    let canvas = surface.canvas();
    canvas.clear(parse_color(&plan.background));

    for item in &plan.items {
        draw_item(canvas, item);
    }
}

fn draw_item(canvas: &skia_safe::Canvas, item: &RenderItem) {
    match &item.kind {
        RenderKind::Rectangle(primitive) => {
            maybe_draw_shadow_rect(canvas, item, primitive.bounds);
            let paint = paint_for_fill(item, primitive.fill.as_deref());
            let rect = sk_rect(primitive.bounds);
            if primitive.corner_radius > 0.0 {
                let rrect = RRect::new_rect_xy(
                    rect,
                    primitive.corner_radius as f32,
                    primitive.corner_radius as f32,
                );
                canvas.draw_rrect(rrect, &paint);
            } else {
                canvas.draw_rect(rect, &paint);
            }
        }
        RenderKind::Ellipse(primitive) => draw_ellipse(canvas, item, primitive),
        RenderKind::Path(primitive) => draw_path(canvas, item, primitive),
        RenderKind::Text(primitive) => draw_text(canvas, item, primitive),
        RenderKind::ImageLayer(_primitive) => {
            // Image loading and sampling will be wired in once the resource layer grows past metadata.
        }
    }
}

fn draw_ellipse(canvas: &skia_safe::Canvas, item: &RenderItem, primitive: &EllipsePrimitive) {
    maybe_draw_shadow_oval(canvas, item, primitive.bounds);
    let paint = paint_for_fill(item, primitive.fill.as_deref());
    canvas.draw_oval(sk_rect(primitive.bounds), &paint);
}

fn draw_path(canvas: &skia_safe::Canvas, item: &RenderItem, primitive: &PathPrimitive) {
    if primitive.points.is_empty() {
        return;
    }

    let mut path = PathBuilder::new();
    let first = primitive.points[0];
    path.move_to((first.x as f32, first.y as f32));

    for point in primitive.points.iter().skip(1) {
        path.line_to((point.x as f32, point.y as f32));
    }

    if primitive.closed {
        path.close();
    }

    let path = path.detach();
    maybe_draw_shadow_path(canvas, item, &path);
    let paint = paint_for_fill(item, primitive.fill.as_deref());
    canvas.draw_path(&path, &paint);
}

fn draw_text(canvas: &skia_safe::Canvas, item: &RenderItem, primitive: &TextPrimitive) {
    maybe_draw_shadow_text(canvas, item, primitive);
    let paint = paint_for_fill(item, primitive.fill.as_deref());
    let font = resolve_font(primitive);
    canvas.draw_str(
        primitive.text.as_str(),
        (primitive.origin.x as f32, primitive.origin.y as f32),
        &font,
        &paint,
    );
}

fn paint_for_fill(item: &RenderItem, fill: Option<&str>) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    let color = with_opacity(
        fill.map(parse_color_str).unwrap_or(Color::BLACK),
        item.opacity,
    );
    paint.set_color(color);
    if let Some(blur_radius) = item.effects.blur_radius {
        if blur_radius > 0.0 {
            paint.set_mask_filter(MaskFilter::blur(
                BlurStyle::Normal,
                blur_radius as f32,
                None,
            ));
        }
    }
    paint
}

fn shadow_paint(item: &RenderItem, shadow: &RenderShadow) -> Paint {
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(with_opacity(parse_color_str(&shadow.color), item.opacity));
    if shadow.blur_radius > 0.0 {
        paint.set_mask_filter(MaskFilter::blur(
            BlurStyle::Normal,
            shadow.blur_radius as f32,
            None,
        ));
    }
    paint
}

fn maybe_draw_shadow_rect(canvas: &skia_safe::Canvas, item: &RenderItem, bounds: crate::Rect) {
    let Some(shadow) = &item.effects.shadow else {
        return;
    };
    let paint = shadow_paint(item, shadow);
    let rect = sk_rect(offset_rect(bounds, shadow.offset_x, shadow.offset_y));
    canvas.draw_rect(rect, &paint);
}

fn maybe_draw_shadow_oval(canvas: &skia_safe::Canvas, item: &RenderItem, bounds: crate::Rect) {
    let Some(shadow) = &item.effects.shadow else {
        return;
    };
    let paint = shadow_paint(item, shadow);
    let rect = sk_rect(offset_rect(bounds, shadow.offset_x, shadow.offset_y));
    canvas.draw_oval(rect, &paint);
}

fn maybe_draw_shadow_path(canvas: &skia_safe::Canvas, item: &RenderItem, path: &skia_safe::Path) {
    let Some(shadow) = &item.effects.shadow else {
        return;
    };
    let paint = shadow_paint(item, shadow);
    canvas.save();
    canvas.translate((shadow.offset_x as f32, shadow.offset_y as f32));
    canvas.draw_path(path, &paint);
    canvas.restore();
}

fn maybe_draw_shadow_text(
    canvas: &skia_safe::Canvas,
    item: &RenderItem,
    primitive: &TextPrimitive,
) {
    let Some(shadow) = &item.effects.shadow else {
        return;
    };
    let paint = shadow_paint(item, shadow);
    let font = resolve_font(primitive);
    canvas.draw_str(
        primitive.text.as_str(),
        (
            (primitive.origin.x + shadow.offset_x) as f32,
            (primitive.origin.y + shadow.offset_y) as f32,
        ),
        &font,
        &paint,
    );
}

fn resolve_font(primitive: &TextPrimitive) -> Font {
    let mut font = Font::default();
    font.set_size(primitive.font_size as f32);

    let font_mgr = FontMgr::new();
    let typeface = primitive
        .font_family
        .as_deref()
        .and_then(|family| font_mgr.match_family_style(family, FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Arial", FontStyle::normal()))
        .or_else(|| font_mgr.match_family_style("Helvetica", FontStyle::normal()));

    if let Some(typeface) = typeface {
        font.set_typeface(typeface);
    }

    font
}

fn parse_color(background: &RenderBackground) -> Color {
    parse_color_str(&background.color)
}

fn parse_color_str(input: &str) -> Color {
    let hex = input.strip_prefix('#').unwrap_or(input);
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok();
            let g = u8::from_str_radix(&hex[2..4], 16).ok();
            let b = u8::from_str_radix(&hex[4..6], 16).ok();
            match (r, g, b) {
                (Some(r), Some(g), Some(b)) => Color::from_argb(255, r, g, b),
                _ => Color::MAGENTA,
            }
        }
        8 => {
            let a = u8::from_str_radix(&hex[0..2], 16).ok();
            let r = u8::from_str_radix(&hex[2..4], 16).ok();
            let g = u8::from_str_radix(&hex[4..6], 16).ok();
            let b = u8::from_str_radix(&hex[6..8], 16).ok();
            match (a, r, g, b) {
                (Some(a), Some(r), Some(g), Some(b)) => Color::from_argb(a, r, g, b),
                _ => Color::MAGENTA,
            }
        }
        _ => Color::MAGENTA,
    }
}

fn sk_rect(rect: crate::Rect) -> SkRect {
    SkRect::from_xywh(
        rect.x as f32,
        rect.y as f32,
        rect.width as f32,
        rect.height as f32,
    )
}

fn offset_rect(rect: crate::Rect, dx: f64, dy: f64) -> crate::Rect {
    crate::Rect {
        x: rect.x + dx,
        y: rect.y + dy,
        width: rect.width,
        height: rect.height,
    }
}

fn with_opacity(color: Color, opacity: f64) -> Color {
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::from_argb(alpha, color.r(), color.g(), color.b())
}

#[cfg(test)]
mod tests {
    use scene_schema::parse_scene_str;

    use crate::build_render_plan;

    use super::{render_plan_to_png, write_plan_png};

    const BASIC_POSTER: &str = include_str!("../../../examples/basic_poster.vsd.json");

    #[test]
    fn skia_backend_exports_png() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let plan = build_render_plan(&scene);
        let png = render_plan_to_png(&plan, 1600, 900).expect("png export should succeed");

        assert!(!png.is_empty());
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn skia_backend_writes_png_file() {
        let scene = parse_scene_str(BASIC_POSTER).expect("scene should parse");
        let plan = build_render_plan(&scene);
        let path = std::env::temp_dir().join("tweaky-skia-export-test.png");

        write_plan_png(&plan, 1600, 900, &path).expect("png file write should succeed");

        let bytes = std::fs::read(&path).expect("written file should be readable");
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");

        let _ = std::fs::remove_file(path);
    }
}
