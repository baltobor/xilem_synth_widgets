//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, BrushIndex, EventCtx, LayoutCtx, NewWidget, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut, WidgetPod, render_text,
};
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{Affine, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use xilem::masonry::vello::peniko::{Color, Fill};

use xilem::masonry::parley::Layout;
use smallvec::SmallVec;
use tracing::trace_span;

const LABEL_HEIGHT: f64 = 16.0;
const PADDING: f64 = 8.0;
const BORDER_WIDTH: f64 = 0.5;
const CORNER_RADIUS: f64 = 6.0;
const LABEL_FONT_SIZE: f32 = 10.0;

/// Default dark red "anodized aluminium" background.
const DEFAULT_BG: Color = Color::from_rgb8(0x5A, 0x1A, 0x1A);

/// Extract r, g, b components as u8 from a Color.
fn color_rgb(c: Color) -> (u8, u8, u8) {
    let rgba = c.to_rgba8();
    (rgba.r, rgba.g, rgba.b)
}

/// Convert HSL to RGB (all values 0..1).
fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    if s == 0.0 {
        return (l, l, l);
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let hue_to_rgb = |t: f64| {
        let t = ((t % 1.0) + 1.0) % 1.0;
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };
    (hue_to_rgb(h + 1.0 / 3.0), hue_to_rgb(h), hue_to_rgb(h - 1.0 / 3.0))
}

/// sRGB to perceptual luminance (Y) using APCA linearization.
/// Based on the APCA-W3 algorithm by Andrew Somers (Myndex).
/// https://www.researchgate.net/lab/Myndex-Research-Andrew-Somers
/// https://github.com/Myndex
fn srgb_to_y(r: u8, g: u8, b: u8) -> f64 {
    const MAIN_TRC: f64 = 2.4;
    const SR_CO: f64 = 0.2126729;
    const SG_CO: f64 = 0.7151522;
    const SB_CO: f64 = 0.0721750;
    let lin = |c: u8| (c as f64 / 255.0).powf(MAIN_TRC);
    SR_CO * lin(r) + SG_CO * lin(g) + SB_CO * lin(b)
}

/// APCA perceptual contrast (Lc value) between text and background.
/// Negative Lc = light text on dark bg. Positive = dark text on light bg.
/// Based on APCA-W3 by Andrew Somers (Myndex), W3C WCAG 3.0 draft.
/// https://www.researchgate.net/lab/Myndex-Research-Andrew-Somers
/// https://github.com/Myndex
fn apca_contrast(txt_y: f64, bg_y: f64) -> f64 {
    const BLK_THRS: f64 = 0.022;
    const BLK_CLMP: f64 = 1.414;
    const NORM_BG: f64 = 0.56;
    const NORM_TXT: f64 = 0.57;
    const REV_TXT: f64 = 0.62;
    const REV_BG: f64 = 0.65;
    const SCALE_BOW: f64 = 1.14;
    const SCALE_WOB: f64 = 1.14;
    const LO_BOW_OFFSET: f64 = 0.027;
    const LO_WOB_OFFSET: f64 = 0.027;
    const DELTA_Y_MIN: f64 = 0.0005;
    const LO_CLIP: f64 = 0.1;

    let ty = if txt_y > BLK_THRS { txt_y } else { txt_y + (BLK_THRS - txt_y).powf(BLK_CLMP) };
    let by = if bg_y > BLK_THRS { bg_y } else { bg_y + (BLK_THRS - bg_y).powf(BLK_CLMP) };

    if (by - ty).abs() < DELTA_Y_MIN { return 0.0; }

    if by > ty {
        let sapc = (by.powf(NORM_BG) - ty.powf(NORM_TXT)) * SCALE_BOW;
        if sapc < LO_CLIP { 0.0 } else { (sapc - LO_BOW_OFFSET) * 100.0 }
    } else {
        let sapc = (by.powf(REV_BG) - ty.powf(REV_TXT)) * SCALE_WOB;
        if sapc > -LO_CLIP { 0.0 } else { (sapc + LO_WOB_OFFSET) * 100.0 }
    }
}

/// Compute an inverse contrast color for text on the given background.
///
/// Uses HSL hue rotation with contrast-aware lightness and saturation
/// adjustment. The result is then verified against the APCA perceptual
/// contrast model and lightness is boosted if needed.
fn inverse_contrast_color(bg: Color) -> Color {
    let (r8, g8, b8) = color_rgb(bg);
    let r = r8 as f64 / 255.0;
    let g = g8 as f64 / 255.0;
    let b = b8 as f64 / 255.0;

    // RGB to HSL
    let min = r.min(g).min(b);
    let max = r.max(g).max(b);
    let l = (min + max) / 2.0;

    let mut s = 0.0;
    if max > 0.0 || min > 0.0 {
        if l <= 0.5 {
            s = (max - min) / (max + min);
        } else {
            s = (max - min) / (2.0 - max - min);
        }
    }

    let mut h = 0.0;
    if max != min {
        if max == r {
            h = (g - b) / (max - min);
        } else if max == g {
            h = 2.0 + (b - r) / (max - min);
        } else {
            h = 4.0 + (r - g) / (max - min);
        }
    }

    // Rotate hue 180 degrees
    let h_deg = h * 60.0;
    let h2 = ((h_deg + 180.0) % 360.0) / 360.0;

    // Contrast-aware lightness
    let contrast = 0.6;
    let mut l2 = (l * (1.0 - contrast)) / (contrast + 1.0);
    if l < 0.382 && (l - l2).abs() < 0.382 {
        l2 = 1.0 - l2;
        if l2 < 0.5 { l2 = 0.5; }
    }
    // Cap lightness — rich but not washed out
    l2 = l2.min(0.55);

    // Adjust saturation for inverse text.
    // For colorful backgrounds (s > 0.15), produce vivid inverse text.
    // For near-neutral backgrounds, keep text neutral.
    if s > 0.5 {
        s = 1.0 - (s * (1.0 - 0.141592653589));
        s *= 0.9;
    } else if s > 0.15 {
        s = (1.0 - s) * 0.9;
    } else {
        s *= 0.5;
    }

    // Generate candidate color
    let (ro, go, bo) = hsl_to_rgb(h2, s, l2);
    let to_u8 = |v: f64| (v * 255.0).round().clamp(0.0, 255.0) as u8;
    let (cr, cg, cb) = (to_u8(ro), to_u8(go), to_u8(bo));

    // Verify APCA contrast; boost lightness if below threshold
    let bg_y = srgb_to_y(r8, g8, b8);
    let txt_y = srgb_to_y(cr, cg, cb);
    let lc = apca_contrast(txt_y, bg_y);

    // Target |Lc| >= 60 for readable small text
    if lc.abs() < 60.0 {
        // Increase lightness until contrast is sufficient
        let mut adj_l = l2;
        for _ in 0..20 {
            adj_l = (adj_l + 0.05).min(1.0);
            let (ar, ag, ab) = hsl_to_rgb(h2, s, adj_l);
            let (tr, tg, tb) = (to_u8(ar), to_u8(ag), to_u8(ab));
            let adj_lc = apca_contrast(srgb_to_y(tr, tg, tb), bg_y);
            if adj_lc.abs() >= 60.0 {
                return Color::from_rgb8(tr, tg, tb);
            }
        }
        // Fallback: bright white or dark black
        let white_lc = apca_contrast(srgb_to_y(255, 255, 255), bg_y);
        if white_lc.abs() > lc.abs() {
            return Color::from_rgb8(0xEE, 0xEE, 0xEE);
        } else {
            return Color::from_rgb8(0x11, 0x11, 0x11);
        }
    }

    Color::from_rgb8(cr, cg, cb)
}

/// Derive border color from a tint (lighter, semi-transparent).
fn border_from_tint(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(
        (r as u16 + (255 - r as u16) * 40 / 100) as u8,
        (g as u16 + (255 - g as u16) * 40 / 100) as u8,
        (b as u16 + (255 - b as u16) * 40 / 100) as u8,
        0x80,
    )
}

/// A group box container with an embossed label and "anodized aluminium" background.
pub struct GroupBox {
    child: WidgetPod<dyn Widget>,
    label: String,
    bg_color: Color,
    border_color: Color,
    text_layout: Layout<BrushIndex>,
    needs_layout: bool,
    fill: bool,
}

impl GroupBox {
    pub fn new(label: impl Into<String>, child: NewWidget<impl Widget + ?Sized>) -> Self {
        let (r, g, b) = color_rgb(DEFAULT_BG);
        Self {
            child: child.erased().to_pod(),
            label: label.into(),
            bg_color: DEFAULT_BG,
            border_color: border_from_tint(r, g, b),
            text_layout: Layout::new(),
            needs_layout: true,
            fill: false,
        }
    }

    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn child_mut<'t>(this: &'t mut WidgetMut<'_, Self>) -> WidgetMut<'t, dyn Widget> {
        this.ctx.get_mut(&mut this.widget.child)
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: impl Into<String>) {
        this.widget.label = label.into();
        this.widget.needs_layout = true;
        this.ctx.request_layout();
    }

    pub fn set_bg_color(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.bg_color = color;
        this.ctx.request_render();
    }

    pub fn with_fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    pub fn set_fill(this: &mut WidgetMut<'_, Self>, fill: bool) {
        this.widget.fill = fill;
        this.ctx.request_layout();
    }

    pub fn with_tint(mut self, color: Color) -> Self {
        self.bg_color = color;
        let (r, g, b) = color_rgb(color);
        self.border_color = border_from_tint(r, g, b);
        self
    }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.bg_color = color;
        let (r, g, b) = color_rgb(color);
        this.widget.border_color = border_from_tint(r, g, b);
        this.ctx.request_render();
    }
}

impl Widget for GroupBox {
    type Action = ();

    fn on_pointer_event(
        &mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent,
    ) {}

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn layout(
        &mut self, ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints,
    ) -> Size {
        // Build label text layout
        if self.needs_layout || ctx.fonts_changed() {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            let mut builder = layout_ctx.ranged_builder(font_ctx, &self.label, 1.0, true);
            builder.push_default(StyleProperty::FontSize(LABEL_FONT_SIZE));
            builder.push_default(StyleProperty::Brush(BrushIndex(0)));
            builder.build_into(&mut self.text_layout, &self.label);
            self.text_layout.break_all_lines(None);
            self.needs_layout = false;
        }

        let min_child_w = (bc.min().width - PADDING * 2.0).max(0.0);
        let min_child_h = (bc.min().height - LABEL_HEIGHT - PADDING * 2.0).max(0.0);
        let max_child_w = (bc.max().width - PADDING * 2.0).max(0.0);
        let max_child_h = (bc.max().height - LABEL_HEIGHT - PADDING * 2.0).max(0.0);
        let child_bc = BoxConstraints::new(
            Size::new(min_child_w, min_child_h),
            Size::new(max_child_w, max_child_h),
        );
        let child_size = ctx.run_layout(&mut self.child, &child_bc);
        ctx.place_child(&mut self.child, Point::new(PADDING, LABEL_HEIGHT + PADDING));

        let content_w = child_size.width.max(min_child_w) + PADDING * 2.0;
        let content_h = child_size.height.max(min_child_h) + LABEL_HEIGHT + PADDING * 2.0;

        // When fill is set, expand width to the maximum available space.
        let w = if self.fill && bc.max().width.is_finite() { bc.max().width } else { content_w };
        bc.constrain(Size::new(w, content_h))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let rect = Rect::from_origin_size(Point::ZERO, size);
        let rr = RoundedRect::from_rect(rect, CORNER_RADIUS);

        // Background
        scene.fill(Fill::NonZero, Affine::IDENTITY, self.bg_color, None, &rr);

        // Subtle border
        scene.stroke(&Stroke::new(BORDER_WIDTH), Affine::IDENTITY, self.border_color, None, &rr);

        // Label text using inverse contrast color (always readable).
        let label_color = inverse_contrast_color(self.bg_color);
        let text_h = self.text_layout.height() as f64;
        let text_y = (LABEL_HEIGHT - text_h) / 2.0;
        render_text(
            scene,
            Affine::translate(Vec2::new(PADDING, text_y)),
            &self.text_layout,
            &[label_color.into()],
            true,
        );
    }

    fn accessibility_role(&self) -> Role { Role::Group }

    fn accessibility(
        &mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node,
    ) {
        node.set_label(self.label.clone());
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::from_slice(&[self.child.id()])
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("GroupBox", id = id.trace())
    }
}
