//! This file is part of the xilem_synth_widget project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

use crate::widgets::fader::Fader as FaderWidget;

/// A vertical fader view with logarithmic scale for volume control.
///
/// Emits dB values. Drag vertically to adjust.
pub struct Fader<F> {
    min_db: f64,
    max_db: f64,
    value_db: f64,
    default_db: f64,
    on_change: F,
    tint: Option<xilem::masonry::vello::peniko::Color>,
}

/// Create a vertical fader. Values are in dB. Typical range: -60.0 to 6.0.
///
/// `default_db` is the value restored on double-click.
pub fn fader<State, Action>(
    min_db: f64,
    max_db: f64,
    value_db: f64,
    default_db: f64,
    on_change: impl Fn(&mut State, f64) -> Action + Send + Sync + 'static,
) -> Fader<impl Fn(&mut State, f64) -> Action + Send + Sync + 'static> {
    Fader { min_db, max_db, value_db, default_db, on_change, tint: None }
}

impl<F> Fader<F> {
    pub fn tint(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl<F> ViewMarker for Fader<F> {}

impl<F, State, Action> View<State, Action, ViewCtx> for Fader<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    type Element = Pod<FaderWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = FaderWidget::new(self.min_db, self.max_db, self.value_db, self.default_db);
        if let Some(c) = self.tint { w = w.with_tint(c); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self, prev: &Self, _: &mut (), _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>, _: &mut State,
    ) {
        if prev.value_db != self.value_db { FaderWidget::set_value_db(&mut element, self.value_db); }
        if prev.min_db != self.min_db || prev.max_db != self.max_db {
            FaderWidget::set_range(&mut element, self.min_db, self.max_db);
        }
        if prev.tint != self.tint {
            if let Some(c) = self.tint { FaderWidget::set_tint(&mut element, c); }
        }
    }

    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self, _: &mut (), message: &mut MessageContext,
        _: Mut<'_, Self::Element>, state: &mut State,
    ) -> MessageResult<Action> {
        if message.take_first().is_some() { return MessageResult::Stale; }
        match message.take_message::<f64>() {
            Some(val) => MessageResult::Action((self.on_change)(state, *val)),
            None => MessageResult::Stale,
        }
    }
}
