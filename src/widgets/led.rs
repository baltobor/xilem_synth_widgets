//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, Circle, Point, Size};
use xilem::masonry::layout::LenReq;
use xilem::masonry::peniko::Fill;
use xilem::Color;

use smallvec::SmallVec;
use tracing::trace_span;

use crate::theme::DEFAULT_TINT;

const LED_RADIUS: f64 = 5.0;
const LED_SIZE: f64 = LED_RADIUS * 2.0 + 4.0;
const OFF_COLOR_R: u8 = 0x40;
const OFF_COLOR_G: u8 = 0x40;
const OFF_COLOR_B: u8 = 0x40;
const BORDER_COLOR: Color = Color::from_rgb8(0x60, 0x60, 0x60);

/// A small LED indicator — a filled circle showing on/off state.
///
/// Active: shows the tint color (default orange).
/// Inactive: shows dark gray.
/// No user interaction — display only.
pub struct Led {
    active: bool,
    tint: Color,
}

impl Led {
    pub fn new(active: bool) -> Self {
        Self { active, tint: DEFAULT_TINT }
    }

    pub fn with_tint(mut self, color: Color) -> Self {
        self.tint = color;
        self
    }

    pub fn set_active(this: &mut WidgetMut<'_, Self>, active: bool) {
        if this.widget.active != active {
            this.widget.active = active;
            this.ctx.request_render();
        }
    }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.tint = color;
        this.ctx.request_render();
    }
}

impl Widget for Led {
    type Action = ();

    fn on_pointer_event(&mut self, _: &mut EventCtx<'_>, _: &mut PropertiesMut<'_>, _: &PointerEvent) {}
    fn accepts_pointer_interaction(&self) -> bool { false }
    fn accepts_focus(&self) -> bool { false }
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn update(&mut self, _: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, _: &Update) {}

    fn measure(
        &mut self, _: &mut MeasureCtx<'_>, _: &PropertiesRef<'_>,
        _axis: Axis, _: LenReq, _: Option<f64>,
    ) -> f64 {
        LED_SIZE
    }

    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let size = ctx.content_box_size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let r = LED_RADIUS;

        let color = if self.active { self.tint } else {
            Color::from_rgb8(OFF_COLOR_R, OFF_COLOR_G, OFF_COLOR_B)
        };

        // Filled circle (main body)
        let circle = Circle::new(Point::new(cx, cy), r);
        painter.fill(&circle, color).fill_rule(Fill::NonZero).draw();

        // Border circle
        let border_stroke = xilem::masonry::kurbo::Stroke::new(0.5);
        painter.stroke(&circle, &border_stroke, BORDER_COLOR).draw();

        // Light spot (white highlight dot for 3D shading effect)
        // Positioned at upper-left of the circle to simulate a light source
        let highlight = Circle::new(Point::new(cx - r * 0.3, cy - r * 0.3), r * 0.25);
        let highlight_color = Color::from_rgba8(0xFF, 0xFF, 0xFF, 0x60); // semi-transparent white
        painter.fill(&highlight, highlight_color).fill_rule(Fill::NonZero).draw();
    }

    fn accessibility_role(&self) -> Role { Role::Image }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, _: &mut Node) {}
    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> { SmallVec::new() }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("Led", id = id.trace())
    }
}
