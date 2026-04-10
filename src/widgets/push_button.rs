//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerButtonEvent, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use xilem::masonry::imaging::Painter;
use xilem::masonry::kurbo::{Axis, Circle, Point, Size, Stroke};
use xilem::masonry::layout::LenReq;
use xilem::masonry::peniko::{Color, Fill};

use smallvec::SmallVec;
use tracing::trace_span;

use crate::theme::DEFAULT_TINT;

const BUTTON_RADIUS: f64 = 8.0;

/// A small circular push button for boolean on/off options.
///
/// When active, shows a lit color. When inactive, shows a dark state.
/// Clicking toggles the state and emits a `bool` action.
pub struct PushButton {
    active: bool,
    lit_color: Color,
}

impl PushButton {
    pub fn new(active: bool) -> Self {
        Self {
            active,
            lit_color: DEFAULT_TINT,
        }
    }

    pub fn with_tint(mut self, color: Color) -> Self {
        self.lit_color = color;
        self
    }

    pub fn set_active(this: &mut WidgetMut<'_, Self>, active: bool) {
        if this.widget.active != active {
            this.widget.active = active;
            this.ctx.request_render();
        }
    }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.lit_color = color;
        this.ctx.request_render();
    }
}

impl Widget for PushButton {
    type Action = bool;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if ctx.is_disabled() {
            return;
        }
        match event {
            PointerEvent::Down(..) => {
                ctx.capture_pointer();
                ctx.request_render();
            }
            PointerEvent::Up(PointerButtonEvent { .. }) => {
                if ctx.is_active() && ctx.is_hovered() {
                    self.active = !self.active;
                    ctx.submit_action::<bool>(self.active);
                    ctx.request_render();
                }
                ctx.release_pointer();
            }
            _ => {}
        }
    }

    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        _axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        BUTTON_RADIUS * 2.0 + 4.0
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &PropertiesRef<'_>,
        _size: Size,
    ) {
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let size = ctx.content_box_size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;

        let circle = Circle::new(Point::new(cx, cy), BUTTON_RADIUS);

        // Outer ring
        let ring_color = Color::from_rgb8(0x60, 0x60, 0x60);
        painter.stroke(circle, &Stroke::new(1.5), ring_color).draw();

        // Fill based on state
        let fill_color = if self.active {
            self.lit_color
        } else if ctx.is_active() {
            Color::from_rgb8(0x50, 0x50, 0x50)
        } else if ctx.is_hovered() {
            Color::from_rgb8(0x45, 0x45, 0x45)
        } else {
            Color::from_rgb8(0x38, 0x38, 0x38)
        };

        let inner = Circle::new(Point::new(cx, cy), BUTTON_RADIUS - 1.5);
        painter.fill(inner, fill_color).fill_rule(Fill::NonZero).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_toggled(if self.active {
            xilem::masonry::accesskit::Toggled::True
        } else {
            xilem::masonry::accesskit::Toggled::False
        });
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("PushButton", id = id.trace())
    }
}
