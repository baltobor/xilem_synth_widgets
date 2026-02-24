//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

use crate::widgets::scope::Scope as ScopeWidget;

pub use crate::widgets::scope::{ScopeBuffer, ScopeSource};

/// An oscilloscope view that displays audio waveforms.
///
/// Accepts a `ScopeSource` for lock-free polling of audio data
/// via animation frames, independent of Xilem's rebuild cycle.
///
/// # Data pipeline
///
/// The scope bypasses Xilem's normal view rebuild cycle because audio
/// data arrives continuously from the DSP thread at audio rate:
///
/// 1. **DSP thread** writes sample buffers into a `triple_buffer::Input`.
/// 2. A [`ScopeSource`] wraps the corresponding `triple_buffer::Output`
///    behind an `Arc<Mutex<..>>` so it can be cheaply cloned and shared.
/// 3. Pass the source to this view: `scope(Some(dsp.scope_source()))`.
/// 4. On first render the widget calls `request_anim_frame()`. On every
///    animation frame (~60 fps) it polls the `ScopeSource` for new data,
///    ingests it (trigger detection + decimation), and requests a repaint.
/// 5. When the `ScopeSource` is replaced (e.g. audio device change), the
///    view detects the new source ID during rebuild and hands it to the
///    widget, which restarts the animation loop.
///
/// This means the scope updates at display refresh rate without forcing
/// Xilem to rebuild the entire view tree on every audio buffer.
pub struct Scope {
    source: Option<ScopeSource>,
    wave_color: Option<xilem::masonry::vello::peniko::Color>,
}

/// Create an oscilloscope view.
///
/// Pass a [`ScopeSource`] obtained from your DSP handle to enable
/// continuous waveform display. The widget polls the source at ~60 fps
/// via animation frames — no manual buffer forwarding needed.
///
/// ```ignore
/// // In your app_logic:
/// scope(Some(state.dsp.scope_source()))
/// ```
pub fn scope(source: Option<ScopeSource>) -> Scope {
    Scope {
        source,
        wave_color: None,
    }
}

impl Scope {
    pub fn wave_color(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.wave_color = Some(color);
        self
    }
}

impl ViewMarker for Scope {}

impl<State, Action> View<State, Action, ViewCtx> for Scope
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<ScopeWidget>;
    /// Tracks the source ID to detect replacement.
    type ViewState = u64;

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = ScopeWidget::new();
        if let Some(c) = self.wave_color {
            w = w.with_wave_color(c);
        }
        let source_id = if let Some(ref src) = self.source {
            w = w.with_source(src.clone());
            src.id()
        } else {
            0
        };
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, source_id)
    }

    fn rebuild(
        &self,
        _prev: &Self,
        view_state: &mut Self::ViewState,
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        let source_id = self.source.as_ref().map_or(0, |s| s.id());
        if source_id != *view_state {
            if let Some(ref src) = self.source {
                ScopeWidget::set_source(&mut element, src.clone());
            }
            *view_state = source_id;
        }
    }

    fn teardown(&self, _: &mut Self::ViewState, ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut Self::ViewState,
        _message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        _: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale
    }
}
