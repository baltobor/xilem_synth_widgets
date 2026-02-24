//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerButtonEvent, PointerEvent,
    PointerUpdate, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId,
    WidgetMut,
};
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{
    Affine, Cap, Line, Point, Rect, RoundedRect, Size, Stroke,
};
use xilem::masonry::vello::peniko::{Color, Fill};

use smallvec::SmallVec;
use tracing::trace_span;

use crate::theme::DEFAULT_TINT;

const FADER_WIDTH: f64 = 32.0;
const FADER_HEIGHT: f64 = 140.0;
const TRACK_WIDTH: f64 = 4.0;
const GRIP_WIDTH: f64 = 24.0;
const GRIP_HEIGHT: f64 = 14.0;
const GRIP_RADIUS: f64 = 3.0;
const TRACK_MARGIN: f64 = GRIP_HEIGHT / 2.0 + 4.0;

/// A vertical fader (slider) with a grip knob and logarithmic scale.
///
/// Designed for volume control. The logarithmic mapping means small
/// movements at the top produce fine dB adjustments while the bottom
/// range covers the full attenuation sweep.
pub struct Fader {
    value: f64,
    min_db: f64,
    max_db: f64,
    default_db: f64,
    tint: Color,
    drag_start_y: Option<f64>,
    drag_start_value: f64,
}

impl Fader {
    pub fn new(min_db: f64, max_db: f64, value_db: f64, default_db: f64) -> Self {
        let norm = Self::db_to_normalized(value_db, min_db, max_db);
        Self {
            value: norm,
            min_db,
            max_db,
            default_db: default_db.clamp(min_db, max_db),
            tint: DEFAULT_TINT,
            drag_start_y: None,
            drag_start_value: 0.0,
        }
    }

    pub fn set_value_db(this: &mut WidgetMut<'_, Self>, value_db: f64) {
        let norm = Self::db_to_normalized(value_db, this.widget.min_db, this.widget.max_db);
        if (this.widget.value - norm).abs() > f64::EPSILON {
            this.widget.value = norm;
            this.ctx.request_render();
        }
    }

    pub fn set_range(this: &mut WidgetMut<'_, Self>, min_db: f64, max_db: f64) {
        this.widget.min_db = min_db;
        this.widget.max_db = max_db;
        this.ctx.request_render();
    }

    pub fn with_tint(mut self, color: Color) -> Self { self.tint = color; self }

    pub fn set_tint(this: &mut WidgetMut<'_, Self>, color: Color) {
        this.widget.tint = color;
        this.ctx.request_render();
    }

    /// Convert dB to normalized 0..1 using linear dB scaling.
    /// Professional faders use linear dB: fader position is proportional to dB.
    /// Since dB = 20*log10(amplitude), this already provides the correct
    /// logarithmic taper for perceptually uniform volume control.
    fn db_to_normalized(db: f64, min_db: f64, max_db: f64) -> f64 {
        let range = max_db - min_db;
        if range.abs() < f64::EPSILON {
            return 0.0;
        }
        ((db - min_db) / range).clamp(0.0, 1.0)
    }

    /// Convert normalized 0..1 back to dB.
    fn normalized_to_db(norm: f64, min_db: f64, max_db: f64) -> f64 {
        min_db + norm * (max_db - min_db)
    }

    fn current_db(&self) -> f64 {
        Self::normalized_to_db(self.value, self.min_db, self.max_db)
    }

    fn track_range(height: f64) -> (f64, f64) {
        (TRACK_MARGIN, height - TRACK_MARGIN)
    }

    fn grip_y(&self, height: f64) -> f64 {
        let (top, bottom) = Self::track_range(height);
        bottom - self.value * (bottom - top)
    }

    #[allow(dead_code)]
    fn y_to_normalized(y: f64, height: f64) -> f64 {
        let (top, bottom) = Self::track_range(height);
        ((bottom - y) / (bottom - top)).clamp(0.0, 1.0)
    }
}

impl Widget for Fader {
    type Action = f64;

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
            PointerEvent::Down(PointerButtonEvent { state, .. }) => {
                ctx.request_focus();
                if state.count == 2 {
                    // Double-click: reset to default
                    self.value = Self::db_to_normalized(self.default_db, self.min_db, self.max_db);
                    ctx.submit_action::<f64>(self.default_db);
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
                        let height = ctx.size().height;
                        let (top, bottom) = Self::track_range(height);
                        let dy = start_y - pos.y;
                        let range = bottom - top;
                        let delta = dy / range;
                        let new_val = (self.drag_start_value + delta).clamp(0.0, 1.0);
                        if (self.value - new_val).abs() > f64::EPSILON {
                            self.value = new_val;
                            let db = self.current_db();
                            ctx.submit_action::<f64>(db);
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

    fn accepts_pointer_interaction(&self) -> bool {
        true
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, _ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &Update) {}

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(FADER_WIDTH, FADER_HEIGHT))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
        let cx = size.width / 2.0;
        let (track_top, track_bottom) = Self::track_range(size.height);

        // Track groove
        let track_rect = Rect::new(
            cx - TRACK_WIDTH / 2.0,
            track_top,
            cx + TRACK_WIDTH / 2.0,
            track_bottom,
        );
        let track_rr = RoundedRect::from_rect(track_rect, TRACK_WIDTH / 2.0);
        scene.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            Color::from_rgb8(0x30, 0x30, 0x30),
            None,
            &track_rr,
        );

        // dB scale marks
        let mark_color = Color::from_rgb8(0x70, 0x70, 0x70);
        let mark_stroke = Stroke::new(1.0);
        let db_marks = [0.0, -6.0, -24.0, -48.0];
        for &db in &db_marks {
            if db >= self.min_db && db <= self.max_db {
                let norm = Self::db_to_normalized(db, self.min_db, self.max_db);
                let y = track_bottom - norm * (track_bottom - track_top);
                let left = cx - TRACK_WIDTH / 2.0 - 6.0;
                let right = cx - TRACK_WIDTH / 2.0 - 2.0;
                scene.stroke(
                    &mark_stroke,
                    Affine::IDENTITY,
                    mark_color,
                    None,
                    &Line::new(Point::new(left, y), Point::new(right, y)),
                );
            }
        }

        // Prominent default mark at -12 dB
        {
            let norm = Self::db_to_normalized(self.default_db, self.min_db, self.max_db);
            let y = track_bottom - norm * (track_bottom - track_top);
            let default_color = Color::from_rgb8(0xB0, 0xB0, 0xB0);
            let default_stroke = Stroke::new(1.5);
            // Left tick
            scene.stroke(
                &default_stroke, Affine::IDENTITY, default_color, None,
                &Line::new(Point::new(cx - TRACK_WIDTH / 2.0 - 8.0, y),
                           Point::new(cx - TRACK_WIDTH / 2.0 - 1.0, y)),
            );
            // Right tick
            scene.stroke(
                &default_stroke, Affine::IDENTITY, default_color, None,
                &Line::new(Point::new(cx + TRACK_WIDTH / 2.0 + 1.0, y),
                           Point::new(cx + TRACK_WIDTH / 2.0 + 8.0, y)),
            );
        }

        // Lit fill from bottom to grip
        let grip_y = self.grip_y(size.height);
        if grip_y < track_bottom {
            let lit_rect = Rect::new(
                cx - TRACK_WIDTH / 2.0 + 0.5,
                grip_y,
                cx + TRACK_WIDTH / 2.0 - 0.5,
                track_bottom,
            );
            scene.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                self.tint,
                None,
                &lit_rect,
            );
        }

        // Grip knob
        let grip_rect = Rect::new(
            cx - GRIP_WIDTH / 2.0,
            grip_y - GRIP_HEIGHT / 2.0,
            cx + GRIP_WIDTH / 2.0,
            grip_y + GRIP_HEIGHT / 2.0,
        );
        let grip_rr = RoundedRect::from_rect(grip_rect, GRIP_RADIUS);
        let grip_color = if ctx.is_active() {
            Color::from_rgb8(0x90, 0x90, 0x90)
        } else if ctx.is_hovered() {
            Color::from_rgb8(0x80, 0x80, 0x80)
        } else {
            Color::from_rgb8(0x6A, 0x6A, 0x6A)
        };
        scene.fill(Fill::NonZero, Affine::IDENTITY, grip_color, None, &grip_rr);
        scene.stroke(
            &Stroke::new(1.0),
            Affine::IDENTITY,
            Color::from_rgb8(0xA0, 0xA0, 0xA0),
            None,
            &grip_rr,
        );

        // Grip lines (texture)
        let line_stroke = Stroke::new(0.5).with_caps(Cap::Butt);
        let line_color = Color::from_rgb8(0x50, 0x50, 0x50);
        for i in [-2.0, 0.0, 2.0] {
            let y = grip_y + i;
            scene.stroke(
                &line_stroke,
                Affine::IDENTITY,
                line_color,
                None,
                &Line::new(
                    Point::new(cx - GRIP_WIDTH / 2.0 + 4.0, y),
                    Point::new(cx + GRIP_WIDTH / 2.0 - 4.0, y),
                ),
            );
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        let db = self.current_db();
        node.set_numeric_value(db);
        node.set_min_numeric_value(self.min_db);
        node.set_max_numeric_value(self.max_db);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("Fader", id = id.trace())
    }
}
