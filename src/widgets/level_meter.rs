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
use xilem::masonry::kurbo::{Axis, Rect, Size};
use xilem::masonry::layout::LenReq;
use xilem::masonry::peniko::Fill;
use xilem::Color;

use smallvec::SmallVec;
use tracing::trace_span;

/// Orientation of the level meter.
#[derive(Clone, Copy, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// Visual style for the level meter bar.
#[derive(Clone, Copy, PartialEq)]
pub enum MeterStyle {
    /// Three-zone gradient: green (low), orange (mid), red (high).
    /// Thresholds are based on dB mapping within the value range.
    Gradient,
    /// Single solid color for the entire filled portion.
    Tint(Color),
}

impl Default for MeterStyle {
    fn default() -> Self { MeterStyle::Gradient }
}

const METER_WIDTH: f64 = 120.0;
const METER_HEIGHT: f64 = 6.0;
const BG_COLOR: Color = Color::from_rgb8(0x20, 0x20, 0x20);
const GREEN: Color = Color::from_rgb8(0x30, 0xC0, 0x30);
const ORANGE: Color = Color::from_rgb8(0xFF, 0x8C, 0x00);
const RED: Color = Color::from_rgb8(0xE0, 0x20, 0x20);

/// A power bar / level meter.
///
/// Shows a colored bar proportional to the value within min..max range.
///
/// Two styles:
/// - `Gradient` (default): three-zone coloring — green, orange, red
/// - `Tint(color)`: single solid color for the entire bar
///
/// Can be horizontal (for transport bar) or vertical (for channel strips).
pub struct LevelMeter {
    value: f64,
    min: f64,
    max: f64,
    orientation: Orientation,
    style: MeterStyle,
}

impl LevelMeter {
    pub fn new(value: f64, min: f64, max: f64, orientation: Orientation) -> Self {
        Self { value, min, max, orientation, style: MeterStyle::Gradient }
    }

    /// Set the visual style (gradient or tint).
    pub fn with_style(mut self, style: MeterStyle) -> Self {
        self.style = style;
        self
    }

    /// Convenience: set a single solid tint color.
    pub fn with_tint(mut self, color: Color) -> Self {
        self.style = MeterStyle::Tint(color);
        self
    }

    pub fn set_value(this: &mut WidgetMut<'_, Self>, value: f64) {
        if (this.widget.value - value).abs() > 0.001 {
            this.widget.value = value;
            this.ctx.request_render();
        }
    }

    pub fn set_style(this: &mut WidgetMut<'_, Self>, style: MeterStyle) {
        if this.widget.style != style {
            this.widget.style = style;
            this.ctx.request_render();
        }
    }

    pub fn set_range(this: &mut WidgetMut<'_, Self>, min: f64, max: f64) {
        this.widget.min = min;
        this.widget.max = max;
        this.ctx.request_render();
    }

    fn normalized(&self) -> f64 {
        let range = self.max - self.min;
        if range.abs() < f64::EPSILON { return 0.0; }
        ((self.value - self.min) / range).clamp(0.0, 1.0)
    }
}

impl Widget for LevelMeter {
    type Action = ();

    fn on_pointer_event(&mut self, _: &mut EventCtx<'_>, _: &mut PropertiesMut<'_>, _: &PointerEvent) {}
    fn accepts_pointer_interaction(&self) -> bool { false }
    fn accepts_focus(&self) -> bool { false }
    fn register_children(&mut self, _: &mut RegisterCtx<'_>) {}
    fn update(&mut self, _: &mut UpdateCtx<'_>, _: &mut PropertiesMut<'_>, _: &Update) {}

    fn measure(
        &mut self, _: &mut MeasureCtx<'_>, _: &PropertiesRef<'_>,
        axis: Axis, _: LenReq, _: Option<f64>,
    ) -> f64 {
        match (self.orientation, axis) {
            (Orientation::Horizontal, Axis::Horizontal) => METER_WIDTH,
            (Orientation::Horizontal, Axis::Vertical) => METER_HEIGHT,
            (Orientation::Vertical, Axis::Horizontal) => METER_HEIGHT,
            (Orientation::Vertical, Axis::Vertical) => METER_WIDTH,
        }
    }

    fn layout(&mut self, _: &mut LayoutCtx<'_>, _: &PropertiesRef<'_>, _: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let size = ctx.content_box_size();
        let norm = self.normalized();

        // Background
        let bg_rect = Rect::new(0.0, 0.0, size.width, size.height);
        painter.fill(&bg_rect, BG_COLOR).fill_rule(Fill::NonZero).draw();

        if norm < 0.001 { return; }

        // Thresholds for three-zone coloring (dB-based when min=-60, max=6)
        let range = self.max - self.min;
        let threshold_norm = ((-12.0 - self.min) / range).clamp(0.0, 1.0);
        let zero_norm = ((0.0 - self.min) / range).clamp(0.0, 1.0);

        match self.orientation {
            Orientation::Horizontal => {
                let fill_w = norm * size.width;

                match self.style {
                    MeterStyle::Tint(color) => {
                        let r = Rect::new(0.0, 0.0, fill_w, size.height);
                        painter.fill(&r, color).fill_rule(Fill::NonZero).draw();
                    }
                    MeterStyle::Gradient => {
                        let thresh_x = threshold_norm * size.width;
                        let zero_x = zero_norm * size.width;
                        // Green zone
                        let green_right = fill_w.min(thresh_x);
                        if green_right > 0.0 {
                            let r = Rect::new(0.0, 0.0, green_right, size.height);
                            painter.fill(&r, GREEN).fill_rule(Fill::NonZero).draw();
                        }
                        // Orange zone
                        if fill_w > thresh_x {
                            let orange_right = fill_w.min(zero_x);
                            if orange_right > thresh_x {
                                let r = Rect::new(thresh_x, 0.0, orange_right, size.height);
                                painter.fill(&r, ORANGE).fill_rule(Fill::NonZero).draw();
                            }
                        }
                        // Red zone
                        if fill_w > zero_x {
                            let r = Rect::new(zero_x, 0.0, fill_w, size.height);
                            painter.fill(&r, RED).fill_rule(Fill::NonZero).draw();
                        }
                    }
                }
            }
            Orientation::Vertical => {
                let fill_h = norm * size.height;
                let top = size.height - fill_h;

                match self.style {
                    MeterStyle::Tint(color) => {
                        let r = Rect::new(0.0, top, size.width, size.height);
                        painter.fill(&r, color).fill_rule(Fill::NonZero).draw();
                    }
                    MeterStyle::Gradient => {
                        let thresh_y = size.height - threshold_norm * size.height;
                        let zero_y = size.height - zero_norm * size.height;
                        // Green zone (bottom)
                        let green_top = top.max(thresh_y);
                        if green_top < size.height {
                            let r = Rect::new(0.0, green_top, size.width, size.height);
                            painter.fill(&r, GREEN).fill_rule(Fill::NonZero).draw();
                        }
                        // Orange zone
                        if top < thresh_y {
                            let orange_top = top.max(zero_y);
                            if orange_top < thresh_y {
                                let r = Rect::new(0.0, orange_top, size.width, thresh_y);
                                painter.fill(&r, ORANGE).fill_rule(Fill::NonZero).draw();
                            }
                        }
                        // Red zone (top)
                        if top < zero_y {
                            let r = Rect::new(0.0, top, size.width, zero_y);
                            painter.fill(&r, RED).fill_rule(Fill::NonZero).draw();
                        }
                    }
                }
            }
        }
    }

    fn accessibility_role(&self) -> Role { Role::Meter }
    fn accessibility(&mut self, _: &mut AccessCtx<'_>, _: &PropertiesRef<'_>, node: &mut Node) {
        node.set_numeric_value(self.value);
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
    }

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> { SmallVec::new() }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("LevelMeter", id = id.trace())
    }
}
