//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageCtx, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Color, Pod, ViewCtx};

use crate::widgets::level_meter::{LevelMeter as LevelMeterWidget, Orientation};

/// A power bar / level meter that shows a value in a colored bar.
///
/// Three-zone coloring: green (low), orange (mid), red (high).
/// Can be horizontal or vertical. Display-only (no interaction).
pub struct LevelMeter {
    value: f64,
    min: f64,
    max: f64,
    orientation: Orientation,
    tint: Option<Color>,
}

/// Create a horizontal level meter.
pub fn level_meter(value: f64, min: f64, max: f64) -> LevelMeter {
    LevelMeter { value, min, max, orientation: Orientation::Horizontal, tint: None }
}

impl LevelMeter {
    /// Set to vertical orientation.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Override the default green/orange/red coloring with a single tint.
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl ViewMarker for LevelMeter {}

impl<State, Action> View<State, Action, ViewCtx> for LevelMeter
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<LevelMeterWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = LevelMeterWidget::new(self.value, self.min, self.max, self.orientation);
        if let Some(c) = self.tint { w = w.with_tint(c); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self, prev: &Self, _: &mut (), _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>, _: &mut State,
    ) {
        if prev.value != self.value { LevelMeterWidget::set_value(&mut element, self.value); }
        if prev.min != self.min || prev.max != self.max {
            LevelMeterWidget::set_range(&mut element, self.min, self.max);
        }
    }

    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_action_source(element);
    }

    fn message(
        &self, _: &mut (), _: &mut MessageCtx,
        _: Mut<'_, Self::Element>, _: &mut State,
    ) -> MessageResult<Action> {
        MessageResult::Stale
    }
}
