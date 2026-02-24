//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, BrushIndex, EventCtx, LayoutCtx, PaintCtx, PointerButtonEvent,
    PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, StyleProperty, Update, UpdateCtx,
    Widget, WidgetId, WidgetMut, render_text,
};
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{Affine, Circle, Point, Rect, RoundedRect, Size, Stroke, Vec2};
use xilem::masonry::vello::peniko::{Color, Fill};

use xilem::masonry::parley::Layout;
use smallvec::SmallVec;
use tracing::trace_span;

use crate::theme::DEFAULT_TINT;

const ROW_HEIGHT: f64 = 16.0;
const DOT_RADIUS: f64 = 4.0;
const DOT_MARGIN: f64 = 2.0;
const LABEL_GAP: f64 = 4.0;
const FONT_SIZE: f32 = 11.0;

/// Where to place the text labels relative to the dot indicator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LabelAlign {
    /// All labels on the left, dot on the right
    Left,
    /// All labels on the right, dot on the left
    Right,
    /// Alternating: even rows left, odd rows right (fish grid)
    Alternating,
}

/// A vertical parameter selector with text labels and a dot indicator.
pub struct ParamSelector {
    selected: usize,
    count: usize,
    labels: Vec<String>,
    label_align: LabelAlign,
    tint: Color,
    /// Pre-built text layouts for each label
    text_layouts: Vec<Layout<BrushIndex>>,
    needs_layout: bool,
}

impl ParamSelector {
    pub fn new(labels: Vec<String>, selected: usize, label_align: LabelAlign) -> Self {
        let count = labels.len();
        Self {
            selected: selected.min(count.saturating_sub(1)),
            count,
            labels,
            label_align,
            tint: DEFAULT_TINT,
            text_layouts: Vec::new(),
            needs_layout: true,
        }
    }

    pub fn with_tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    pub fn set_selected(this: &mut WidgetMut<'_, Self>, selected: usize) {
        let s = selected.min(this.widget.count.saturating_sub(1));
        if this.widget.selected != s {
            this.widget.selected = s;
            this.ctx.request_render();
        }
    }

    pub fn set_labels(this: &mut WidgetMut<'_, Self>, labels: Vec<String>) {
        this.widget.count = labels.len();
        this.widget.labels = labels;
        this.widget.selected = this.widget.selected.min(this.widget.count.saturating_sub(1));
        this.widget.needs_layout = true;
        this.ctx.request_layout();
    }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.tint = color;
        this.ctx.request_render();
    }

    fn row_rect(&self, index: usize, size: Size) -> (f64, f64) {
        let y = index as f64 * ROW_HEIGHT;
        (y, (y + ROW_HEIGHT).min(size.height))
    }

    fn hit_test(&self, pos: Point, size: Size) -> Option<usize> {
        for i in 0..self.count {
            let (y0, y1) = self.row_rect(i, size);
            if pos.y >= y0 && pos.y < y1 && pos.x >= 0.0 && pos.x <= size.width {
                return Some(i);
            }
        }
        None
    }

    fn label_on_left(&self, index: usize) -> bool {
        match self.label_align {
            LabelAlign::Left => true,
            LabelAlign::Right => false,
            LabelAlign::Alternating => index % 2 == 0,
        }
    }

    fn dot_col_w() -> f64 {
        DOT_RADIUS * 2.0 + DOT_MARGIN * 2.0
    }
}

impl Widget for ParamSelector {
    type Action = usize;

    fn on_pointer_event(
        &mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent,
    ) {
        if ctx.is_disabled() { return; }
        if let PointerEvent::Up(PointerButtonEvent { state, .. }) = event {
            let pos = ctx.local_position(state.position);
            if let Some(idx) = self.hit_test(pos, ctx.size()) {
                if self.selected != idx {
                    self.selected = idx;
                    ctx.submit_action::<usize>(idx);
                    ctx.request_render();
                }
            }
        }
    }

    fn accepts_pointer_interaction(&self) -> bool { true }
    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}
    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn layout(
        &mut self, ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints,
    ) -> Size {
        // Build text layouts for each label
        if self.needs_layout || ctx.fonts_changed() {
            let (font_ctx, layout_ctx) = ctx.text_contexts();
            self.text_layouts.clear();
            for label in &self.labels {
                let mut builder = layout_ctx.ranged_builder(font_ctx, label, 1.0, true);
                builder.push_default(StyleProperty::FontSize(FONT_SIZE));
                let mut layout = builder.build(label);
                layout.break_all_lines(None);
                self.text_layouts.push(layout);
            }
            self.needs_layout = false;
        }

        // Compute width from actual text widths
        let dot_col_w = Self::dot_col_w();
        let max_text_w = self.text_layouts.iter()
            .map(|l| l.width() as f64)
            .fold(0.0_f64, f64::max);
        let w = max_text_w + dot_col_w + LABEL_GAP;
        let h = self.count as f64 * ROW_HEIGHT;
        bc.constrain(Size::new(w, h))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let dot_col_w = Self::dot_col_w();

        // Capsule frame centered on dot column
        let frame_pad = 2.0;
        let frame_w = DOT_RADIUS * 2.0 + 6.0;
        let dot_center_x = match self.label_align {
            LabelAlign::Left | LabelAlign::Alternating => size.width - dot_col_w / 2.0,
            LabelAlign::Right => dot_col_w / 2.0,
        };
        let frame_rect = Rect::new(
            dot_center_x - frame_w / 2.0, frame_pad,
            dot_center_x + frame_w / 2.0, size.height - frame_pad,
        );
        let frame_rr = RoundedRect::from_rect(frame_rect, frame_w / 2.0);
        scene.fill(Fill::NonZero, Affine::IDENTITY, Color::from_rgb8(0x2A, 0x2A, 0x2A), None, &frame_rr);
        scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(0x55, 0x55, 0x55), None, &frame_rr);

        for i in 0..self.count {
            let (y0, _) = self.row_rect(i, size);
            let cy = y0 + ROW_HEIGHT / 2.0;
            let is_selected = i == self.selected;
            let left = self.label_on_left(i);

            // Dot
            let dot_x = if left { size.width - dot_col_w / 2.0 } else { dot_col_w / 2.0 };
            let center = Point::new(dot_x, cy);
            if is_selected {
                let dot = Circle::new(center, DOT_RADIUS + 1.5);
                scene.fill(Fill::NonZero, Affine::IDENTITY, self.tint, None, &dot);
            }

            // Text label via parley layout
            if let Some(layout) = self.text_layouts.get(i) {
                let text_color = if is_selected {
                    Color::from_rgb8(0xEE, 0xEE, 0xEE)
                } else {
                    Color::from_rgb8(0x99, 0x99, 0x99)
                };

                let text_w = layout.width() as f64;
                let text_h = layout.height() as f64;
                let text_x = if left {
                    size.width - dot_col_w - LABEL_GAP - text_w
                } else {
                    dot_col_w + LABEL_GAP
                };
                let text_y = cy - text_h / 2.0;

                render_text(
                    scene,
                    Affine::translate(Vec2::new(text_x, text_y)),
                    layout,
                    &[text_color.into()],
                    true,
                );
            }
        }
    }

    fn accessibility_role(&self) -> Role { Role::RadioGroup }

    fn accessibility(
        &mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node,
    ) {
        if self.selected < self.labels.len() {
            node.set_description(self.labels[self.selected].clone());
        }
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> { SmallVec::new() }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("ParamSelector", id = id.trace())
    }
}
