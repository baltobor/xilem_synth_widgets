//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use std::sync::{Arc, Mutex};

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use xilem::masonry::kurbo::Axis;
use xilem::masonry::layout::LenReq;
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{
    Affine, BezPath, Cap, Line, Point, Rect, RoundedRect, Size, Stroke,
};
use xilem::masonry::vello::peniko::{Color, Fill};

use tracing::trace_span;

const SCOPE_WIDTH: f64 = 192.0;
const SCOPE_HEIGHT: f64 = 196.0;
const BORDER_RADIUS: f64 = 4.0;
const PADDING: f64 = 2.0;

/// Thread-safe buffer for passing audio samples to the scope.
#[derive(Clone)]
pub struct ScopeBuffer {
    pub samples: Arc<Vec<f32>>,
}

impl ScopeBuffer {
    pub fn new(samples: Vec<f32>) -> Self {
        Self {
            samples: Arc::new(samples),
        }
    }

    pub fn from_arc(samples: Arc<Vec<f32>>) -> Self {
        Self { samples }
    }
}

/// Shared scope data source for lock-free polling from the widget.
#[derive(Clone)]
pub struct ScopeSource {
    inner: Arc<Mutex<triple_buffer::Output<Vec<f32>>>>,
    id: u64,
}

static SCOPE_SOURCE_NEXT_ID: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

impl ScopeSource {
    pub fn new(output: triple_buffer::Output<Vec<f32>>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(output)),
            id: SCOPE_SOURCE_NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Poll for new data. Returns Some if the buffer has been updated.
    pub fn poll(&self) -> Option<ScopeBuffer> {
        let mut out = self.inner.lock().unwrap();
        if out.update() {
            let samples = out.peek_output_buffer();
            if !samples.is_empty() {
                return Some(ScopeBuffer::from_arc(Arc::new(samples.clone())));
            }
        }
        None
    }
}

/// Zero-crossing detection mode
#[derive(Clone, Copy, Debug)]
pub enum TriggerMode {
    /// Trigger on negative-to-positive crossing
    RisingEdge,
    /// Trigger on positive-to-negative crossing
    #[allow(dead_code)]
    FallingEdge,
}

/// An oscilloscope widget that displays audio waveforms.
pub struct Scope {
    display_points: Vec<f32>,
    raw_buffer: Vec<f32>,
    display_width: usize,
    trigger_mode: TriggerMode,
    trigger_threshold: f32,
    wave_color: Color,
    bg_color: Color,
    grid_color: Color,
    generation: u64,
    source: Option<ScopeSource>,
}

impl Scope {
    pub fn new() -> Self {
        let display_w = (SCOPE_WIDTH - PADDING * 2.0) as usize;
        Self {
            display_points: vec![0.0; display_w],
            raw_buffer: Vec::new(),
            display_width: display_w,
            trigger_mode: TriggerMode::RisingEdge,
            trigger_threshold: 0.02,
            wave_color: Color::from_rgb8(0x00, 0xFF, 0x80),
            bg_color: Color::from_rgb8(0x0A, 0x0A, 0x0A),
            grid_color: Color::from_rgb8(0x20, 0x30, 0x20),
            generation: 0,
            source: None,
        }
    }

    pub fn with_source(mut self, source: ScopeSource) -> Self {
        self.source = Some(source);
        self
    }

    pub fn set_source(this: &mut WidgetMut<'_, Self>, source: ScopeSource) {
        this.widget.source = Some(source);
        this.ctx.request_anim_frame();
    }

    pub fn with_wave_color(mut self, color: Color) -> Self {
        self.wave_color = color;
        self
    }

    pub fn with_trigger_threshold(mut self, threshold: f32) -> Self {
        self.trigger_threshold = threshold;
        self
    }

    /// Push a new buffer of samples.
    pub fn push_buffer(this: &mut WidgetMut<'_, Self>, buffer: &ScopeBuffer) {
        if this.widget.ingest_buffer(buffer) {
            this.ctx.request_render();
        }
    }

    fn ingest_buffer(&mut self, buffer: &ScopeBuffer) -> bool {
        let samples = &buffer.samples;
        if samples.is_empty() {
            return false;
        }

        let max_raw = self.display_width * 4;
        self.raw_buffer.extend_from_slice(samples);
        if self.raw_buffer.len() > max_raw {
            let drain = self.raw_buffer.len() - max_raw;
            self.raw_buffer.drain(..drain);
        }

        let trigger_pos = self.find_trigger_point();

        let half = self.display_width / 2;
        let display_start = trigger_pos.saturating_sub(half);
        let display_end = (trigger_pos + half).min(self.raw_buffer.len());
        let span = display_end - display_start;

        if span >= self.display_width {
            let step = span as f64 / self.display_width as f64;
            for i in 0..self.display_width {
                let src_idx = display_start + (i as f64 * step) as usize;
                self.display_points[i] =
                    self.raw_buffer[src_idx.min(self.raw_buffer.len() - 1)];
            }
        } else {
            let offset = (self.display_width - span) / 2;
            for i in 0..self.display_width {
                if i >= offset && i < offset + span {
                    self.display_points[i] = self.raw_buffer[display_start + i - offset];
                } else {
                    self.display_points[i] = 0.0;
                }
            }
        }

        self.generation += 1;
        true
    }

    fn find_trigger_point(&self) -> usize {
        let threshold = self.trigger_threshold;
        let samples = &self.raw_buffer;
        let half = self.display_width / 2;
        let search_start = half;
        let search_end = samples.len().saturating_sub(half);

        if search_start >= search_end || search_end < 2 {
            return half.min(samples.len().saturating_sub(1));
        }

        match self.trigger_mode {
            TriggerMode::RisingEdge => {
                let mut armed = false;
                for i in search_start..search_end {
                    if samples[i] < -threshold {
                        armed = true;
                    }
                    if armed && samples[i] > threshold {
                        return i;
                    }
                }
            }
            TriggerMode::FallingEdge => {
                let mut armed = false;
                for i in search_start..search_end {
                    if samples[i] > threshold {
                        armed = true;
                    }
                    if armed && samples[i] < -threshold {
                        return i;
                    }
                }
            }
        }

        half.min(samples.len().saturating_sub(1))
    }
}

impl Widget for Scope {
    type Action = ();

    fn on_pointer_event(
        &mut self,
        _ctx: &mut EventCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _event: &PointerEvent,
    ) {
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn on_anim_frame(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        _interval: u64,
    ) {
        if let Some(ref source) = self.source {
            if let Some(buf) = source.poll() {
                if self.ingest_buffer(&buf) {
                    ctx.request_render();
                }
            }
            ctx.request_anim_frame();
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        event: &Update,
    ) {
        if matches!(event, Update::WidgetAdded) && self.source.is_some() {
            ctx.request_anim_frame();
        }
    }

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        _len_req: LenReq,
        _cross_length: Option<f64>,
    ) -> f64 {
        match axis {
            Axis::Horizontal => SCOPE_WIDTH,
            Axis::Vertical => SCOPE_HEIGHT,
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {
        ctx.clear_baselines();
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.content_box_size();
        let rect = Rect::from_origin_size(Point::ZERO, size);
        let rr = RoundedRect::from_rect(rect, BORDER_RADIUS);

        // Background
        scene.fill(Fill::NonZero, Affine::IDENTITY, self.bg_color, None, &rr);

        let draw_x = PADDING;
        let draw_y = PADDING;
        let draw_w = size.width - PADDING * 2.0;
        let draw_h = size.height - PADDING * 2.0;
        let mid_y = draw_y + draw_h / 2.0;

        // Grid lines
        let grid_stroke = Stroke::new(0.5);
        // Horizontal center line
        scene.stroke(
            &grid_stroke,
            Affine::IDENTITY,
            self.grid_color,
            None,
            &Line::new(Point::new(draw_x, mid_y), Point::new(draw_x + draw_w, mid_y)),
        );
        // Quarter lines
        for frac in [0.25, 0.75] {
            let y = draw_y + draw_h * frac;
            scene.stroke(
                &grid_stroke,
                Affine::IDENTITY,
                self.grid_color,
                None,
                &Line::new(Point::new(draw_x, y), Point::new(draw_x + draw_w, y)),
            );
        }
        // Vertical center
        let mid_x = draw_x + draw_w / 2.0;
        scene.stroke(
            &grid_stroke,
            Affine::IDENTITY,
            self.grid_color,
            None,
            &Line::new(Point::new(mid_x, draw_y), Point::new(mid_x, draw_y + draw_h)),
        );
        // Vertical quarters
        for frac in [0.25, 0.75] {
            let x = draw_x + draw_w * frac;
            scene.stroke(
                &grid_stroke,
                Affine::IDENTITY,
                self.grid_color,
                None,
                &Line::new(Point::new(x, draw_y), Point::new(x, draw_y + draw_h)),
            );
        }

        // Waveform
        if !self.display_points.is_empty() {
            let mut path = BezPath::new();
            let step = draw_w / self.display_points.len() as f64;

            for (i, &sample) in self.display_points.iter().enumerate() {
                let x = draw_x + i as f64 * step;
                let clamped = sample.clamp(-1.0, 1.0) as f64;
                let y = mid_y - clamped * (draw_h / 2.0 - 2.0);

                if i == 0 {
                    path.move_to(Point::new(x, y));
                } else {
                    path.line_to(Point::new(x, y));
                }
            }

            scene.stroke(
                &Stroke::new(1.5).with_caps(Cap::Round),
                Affine::IDENTITY,
                self.wave_color,
                None,
                &path,
            );
        }

        // Border
        scene.stroke(
            &Stroke::new(0.5),
            Affine::IDENTITY,
            Color::from_rgb8(0x40, 0x40, 0x40),
            None,
            &rr,
        );
    }

    fn accessibility_role(&self) -> Role {
        Role::Canvas
    }

    fn accessibility(
        &mut self,
        _ctx: &mut AccessCtx<'_>,
        _props: &PropertiesRef<'_>,
        node: &mut Node,
    ) {
        node.set_description("Oscilloscope display".to_string());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::default()
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("Scope", id = id.trace())
    }
}
