//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use std::sync::{Arc, Mutex};

use xilem::masonry::accesskit::{Node, Role};
use xilem::masonry::core::{
    AccessCtx, BoxConstraints, EventCtx, LayoutCtx, PaintCtx, PointerEvent, PropertiesMut,
    PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetId, WidgetMut,
};
use xilem::masonry::vello::Scene;
use xilem::masonry::vello::kurbo::{
    Affine, BezPath, Cap, Line, Point, Rect, RoundedRect, Size, Stroke,
};
use xilem::masonry::vello::peniko::{Color, Fill};

use smallvec::SmallVec;
use tracing::trace_span;

const SCOPE_WIDTH: f64 = 192.0;
const SCOPE_HEIGHT: f64 = 196.0;
const BORDER_RADIUS: f64 = 4.0;
const PADDING: f64 = 2.0;

/// Thread-safe buffer for passing audio samples to the scope.
///
/// Wrap your sample data in `Arc<Vec<f32>>` and send it from any thread.
/// The scope will decimate the data for display and only keep the
/// last buffer for rendering efficiency.
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
///
/// Wraps a `triple_buffer::Output<Vec<f32>>` so the Scope widget can
/// poll for new audio data during animation frames without going
/// through Xilem's view rebuild cycle.
///
/// Create one from the triple-buffer output that pairs with your DSP
/// thread's input. Each `ScopeSource` gets a unique ID so the view
/// layer can detect when the source is replaced (e.g. on audio device
/// change). Cloning shares the same underlying buffer and ID.
#[derive(Clone)]
pub struct ScopeSource {
    inner: Arc<Mutex<triple_buffer::Output<Vec<f32>>>>,
    /// Unique ID for detecting source replacement.
    id: u64,
}

static SCOPE_SOURCE_NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

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
///
/// The zero-crossing trigger point is centered in the display: the left
/// half shows the waveform before the crossing, the right half shows
/// after. Changing frequency expands the waveform symmetrically from
/// the center. The trigger search starts at `display_width / 2` into
/// the raw buffer to ensure enough pre-trigger data for the left side.
///
/// Updates independently of Xilem's rebuild cycle by polling a shared
/// `ScopeSource` (triple-buffer output) during `on_anim_frame`.
///
/// Features:
/// - Centered zero-crossing trigger with hysteresis for stable display
/// - Accepts `ScopeSource` for lock-free polling from real-time DSP threads
/// - Decimates data for display (CPU friendly)
/// - ~60fps rendering via animation frames
/// - Fixed 192x196 pixel display area
pub struct Scope {
    /// The display buffer (decimated for rendering)
    display_points: Vec<f32>,
    /// Raw buffer for trigger detection
    raw_buffer: Vec<f32>,
    /// Number of display points to show
    display_width: usize,
    /// Trigger mode
    trigger_mode: TriggerMode,
    /// Hysteresis threshold for zero-crossing (prevents jitter)
    trigger_threshold: f32,
    /// Waveform color
    wave_color: Color,
    /// Background color
    bg_color: Color,
    /// Grid color
    grid_color: Color,
    /// Generation counter to detect new data
    generation: u64,
    /// Optional shared source for polling new data during anim frames
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

    /// Push a new buffer of samples. The scope will find a zero-crossing
    /// trigger point and decimate the data for display.
    pub fn push_buffer(this: &mut WidgetMut<'_, Self>, buffer: &ScopeBuffer) {
        if this.widget.ingest_buffer(buffer) {
            this.ctx.request_render();
        }
    }

    /// Internal: ingest a buffer and return true if display was updated.
    fn ingest_buffer(&mut self, buffer: &ScopeBuffer) -> bool {
        let samples = &buffer.samples;
        if samples.is_empty() {
            return false;
        }

        // Append to raw buffer, keep a reasonable amount for trigger search
        let max_raw = self.display_width * 4;
        self.raw_buffer.extend_from_slice(samples);
        if self.raw_buffer.len() > max_raw {
            let drain = self.raw_buffer.len() - max_raw;
            self.raw_buffer.drain(..drain);
        }

        // Find trigger point (zero-crossing with hysteresis)
        let trigger_pos = self.find_trigger_point();

        // Center the trigger point in the display: show half before, half after.
        let half = self.display_width / 2;
        let display_start = trigger_pos.saturating_sub(half);
        let display_end = (trigger_pos + half).min(self.raw_buffer.len());
        let span = display_end - display_start;

        // Copy exactly the centered span into display buffer (1:1 or decimated)
        if span >= self.display_width {
            // Decimate: map display_width points from the span
            let step = span as f64 / self.display_width as f64;
            for i in 0..self.display_width {
                let src_idx = display_start + (i as f64 * step) as usize;
                self.display_points[i] = self.raw_buffer[src_idx.min(self.raw_buffer.len() - 1)];
            }
        } else {
            // Not enough data: center what we have
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

    /// Find a stable zero-crossing trigger point using hysteresis.
    ///
    /// Looks for a region where samples go from below -threshold
    /// to above +threshold (rising edge) or vice versa.
    fn find_trigger_point(&self) -> usize {
        let threshold = self.trigger_threshold;
        let samples = &self.raw_buffer;
        let half = self.display_width / 2;
        // Start searching from half-display into the buffer so there's
        // enough data before the trigger for the left side of the display.
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

        // Fallback: center of available data
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
        &mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _interval: u64,
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

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) && self.source.is_some() {
            ctx.request_anim_frame();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx<'_>,
        _props: &mut PropertiesMut<'_>,
        bc: &BoxConstraints,
    ) -> Size {
        bc.constrain(Size::new(SCOPE_WIDTH, SCOPE_HEIGHT))
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, scene: &mut Scene) {
        let size = ctx.size();
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
                // Clamp sample to -1..1 range for display
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

    fn children_ids(&self) -> SmallVec<[WidgetId; 16]> {
        SmallVec::new()
    }

    fn make_trace_span(&self, id: WidgetId) -> tracing::Span {
        trace_span!("Scope", id = id.trace())
    }
}
