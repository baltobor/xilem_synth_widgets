//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use std::f64::consts::PI;

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerButtonEvent, PointerEvent,
    PointerUpdate, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut,
};
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{Affine, Arc, Cap, Circle, Line, Point, Size, Stroke, Vec2};
use xilem::masonry::vello::peniko::{Color, Fill};

use smallvec::SmallVec;
use tracing::trace_span;

use crate::theme::DEFAULT_TINT;

const KNOB_RADIUS: f64 = 18.0;
const KNOB_RADIUS_SMALL: f64 = 11.0;
const RING_WIDTH: f64 = 3.0;
const RING_WIDTH_SMALL: f64 = 2.0;
const INDICATOR_WIDTH: f64 = 2.5;
const INDICATOR_WIDTH_SMALL: f64 = 1.5;
const ARC_START: f64 = 0.75 * PI;
const ARC_SWEEP: f64 = 1.5 * PI;

/// A rotary knob widget with a lit color ring showing the value range.
pub struct Knob {
    value: f64,
    min: f64,
    max: f64,
    default: f64,
    step: f64,
    tint: Color,
    small: bool,
    drag_start_y: Option<f64>,
    drag_start_value: f64,
}

impl Knob {
    pub fn new(min: f64, max: f64, value: f64, default: f64) -> Self {
        Self {
            value: value.clamp(min, max),
            min,
            max,
            default: default.clamp(min, max),
            step: 0.0,
            tint: DEFAULT_TINT,
            small: false,
            drag_start_y: None,
            drag_start_value: 0.0,
        }
    }

    pub fn with_step(mut self, step: f64) -> Self { self.step = step; self }
    pub fn with_tint(mut self, color: Color) -> Self { self.tint = color; self }
    pub fn with_small(mut self, small: bool) -> Self { self.small = small; self }

    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        let v = value.clamp(this.widget.min, this.widget.max);
        if (this.widget.value - v).abs() > f64::EPSILON {
            this.widget.value = v;
            this.ctx.request_render();
        }
    }

    pub fn set_range(this: &mut WidgetMut<'_, Self>, min: f64, max: f64) {
        this.widget.min = min;
        this.widget.max = max;
        this.widget.value = this.widget.value.clamp(min, max);
        this.ctx.request_render();
    }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.tint = color;
        this.ctx.request_render();
    }

    fn radius(&self) -> f64 { if self.small { KNOB_RADIUS_SMALL } else { KNOB_RADIUS } }
    fn ring_w(&self) -> f64 { if self.small { RING_WIDTH_SMALL } else { RING_WIDTH } }
    fn indicator_w(&self) -> f64 { if self.small { INDICATOR_WIDTH_SMALL } else { INDICATOR_WIDTH } }

    fn normalized(&self) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON { return 0.0; }
        (self.value - self.min) / (self.max - self.min)
    }

    fn default_normalized(&self) -> f64 {
        if (self.max - self.min).abs() < f64::EPSILON { return 0.0; }
        (self.default - self.min) / (self.max - self.min)
    }

    fn quantize(&self, val: f64) -> f64 {
        if self.step > 0.0 {
            let steps = ((val - self.min) / self.step).round();
            (self.min + steps * self.step).clamp(self.min, self.max)
        } else {
            val.clamp(self.min, self.max)
        }
    }

    fn angle_for_normalized(n: f64) -> f64 {
        ARC_START + n * ARC_SWEEP
    }
}

impl Widget for Knob {
    type Action = f64;

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        if ctx.is_disabled() { return; }
        match event {
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                ctx.request_focus();
                if state.count == 2 {
                    // Double-click: reset to default
                    self.value = self.default;
                    ctx.submit_action::<f64>(self.value);
                    ctx.request_render();
                    return;
                }
                ctx.capture_pointer();
                let pos = ctx.local_position(state.position);
                self.drag_start_y = Some(pos.y);
                self.drag_start_value = self.value;
            }
            PointerEvent::Move(PointerUpdate { current, .. }) => {
                if ctx.is_active() {
                    if let Some(start_y) = self.drag_start_y {
                        let pos = ctx.local_position(current.position);
                        let dy = start_y - pos.y;
                        let sensitivity = 0.005;
                        let range = self.max - self.min;
                        let new_val = self.quantize(self.drag_start_value + dy * sensitivity * range);
                        if (self.value - new_val).abs() > f64::EPSILON {
                            self.value = new_val;
                            ctx.submit_action::<f64>(self.value);
                            ctx.request_render();
                        }
                    }
                }
            }
            PointerEvent::Up(..) => {
                if ctx.is_active() {
                    ctx.release_pointer();
                    self.drag_start_y = None;
                }
            }
            _ => {}
        }
    }

    fn accepts_pointer_interaction(&self) -> bool { true }
    fn accepts_focus(&self) -> bool { true }
    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}
    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &mut PropertiesMut<'_>, bc: &BoxConstraints) -> Size {
        let side = self.radius() * 2.0 + 4.0;
        bc.constrain(Size::new(side, side))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let r = self.radius();
        let ring_w = self.ring_w();

        // Track arc
        let track_arc = Arc::new(Point::new(cx, cy), Vec2::new(r, r), ARC_START, ARC_SWEEP, 0.0);
        scene.stroke(
            &Stroke::new(ring_w).with_caps(Cap::Round),
            Affine::IDENTITY, Color::from_rgb8(0x40, 0x40, 0x40), None, &track_arc,
        );

        // Lit arc from default to current value
        let def_n = self.default_normalized();
        let cur_n = self.normalized();
        if (def_n - cur_n).abs() > 0.001 {
            let start = Self::angle_for_normalized(def_n.min(cur_n));
            let end = Self::angle_for_normalized(def_n.max(cur_n));
            let lit_arc = Arc::new(Point::new(cx, cy), Vec2::new(r, r), start, end - start, 0.0);
            scene.stroke(
                &Stroke::new(ring_w).with_caps(Cap::Round),
                Affine::IDENTITY, self.tint, None, &lit_arc,
            );
        }

        // Body
        let body_r = r - if self.small { 3.5 } else { 5.0 };
        let body = Circle::new(Point::new(cx, cy), body_r);
        let body_color = if ctx.is_active() {
            Color::from_rgb8(0x70, 0x70, 0x70)
        } else if ctx.is_hovered() {
            Color::from_rgb8(0x60, 0x60, 0x60)
        } else {
            Color::from_rgb8(0x50, 0x50, 0x50)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, body_color, None, &body);
        scene.stroke(&Stroke::new(1.0), Affine::IDENTITY, Color::from_rgb8(0x80, 0x80, 0x80), None, &body);

        // Indicator line
        let angle = Self::angle_for_normalized(cur_n);
        let inner_r = body_r * 0.3;
        let outer_r = body_r * 0.85;
        let dir = Vec2::from_angle(angle);
        let p0 = Point::new(cx + dir.x * inner_r, cy + dir.y * inner_r);
        let p1 = Point::new(cx + dir.x * outer_r, cy + dir.y * outer_r);
        scene.stroke(
            &Stroke::new(self.indicator_w()).with_caps(Cap::Round),
            Affine::IDENTITY, Color::WHITE, None, &Line::new(p0, p1),
        );
    }

    fn accessibility_role(&self) -> Role { Role::Slider }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_numeric_value(self.value);
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
        node.set_numeric_value_step(if self.step > 0.0 { self.step } else { 0.01 });
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> { SmallVec::new() }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("Knob", id = id.trace())
    }
}
