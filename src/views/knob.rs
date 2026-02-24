//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

use crate::widgets::knob::Knob as KnobWidget;

/// A rotary knob view for continuous parameter control.
///
/// Drag vertically to adjust the value. The lit arc shows the
/// distance from the default (reference) value.
pub struct Knob<F> {
    min: f64,
    max: f64,
    value: f64,
    default: f64,
    on_change: F,
    step: f64,
    small: bool,
    tint: Option<xilem::masonry::vello::peniko::Color>,
}

/// Create a rotary knob.
///
/// `default` is the reference value - the lit arc shows distance from it.
pub fn knob<State, Action>(
    min: f64,
    max: f64,
    value: f64,
    default: f64,
    on_change: impl Fn(&mut State, f64) -> Action + Send + Sync + 'static,
) -> Knob<impl Fn(&mut State, f64) -> Action + Send + Sync + 'static> {
    Knob { min, max, value, default, on_change, step: 0.0, small: false, tint: None }
}

impl<F> Knob<F> {
    pub fn step(mut self, step: f64) -> Self { self.step = step; self }
    pub fn small(mut self) -> Self { self.small = true; self }

    pub fn tint(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl<F> ViewMarker for Knob<F> {}

impl<F, State, Action> View<State, Action, ViewCtx> for Knob<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, f64) -> Action + Send + Sync + 'static,
{
    type Element = Pod<KnobWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = KnobWidget::new(self.min, self.max, self.value, self.default)
            .with_small(self.small);
        if self.step > 0.0 { w = w.with_step(self.step); }
        if let Some(c) = self.tint { w = w.with_tint(c); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self, prev: &Self, _: &mut (), _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>, _: &mut State,
    ) {
        if prev.value != self.value { KnobWidget::set_value(&mut element, self.value); }
        if prev.min != self.min || prev.max != self.max {
            KnobWidget::set_range(&mut element, self.min, self.max);
        }
        if prev.tint != self.tint {
            if let Some(c) = self.tint { KnobWidget::set_tint(&mut element, c); }
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
