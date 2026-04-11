//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageCtx, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

use crate::widgets::level_meter::{LevelMeter as LevelMeterWidget, MeterStyle, Orientation};

/// A power bar / level meter that shows a value in a colored bar.
///
/// Two styles:
/// - `Gradient` (default): three-zone coloring — green, orange, red
/// - `Tint(color)`: single solid color for the entire bar
///
/// Can be horizontal or vertical. Display-only (no interaction).
pub struct LevelMeter {
    value: f64,
    min: f64,
    max: f64,
    orientation: Orientation,
    style: MeterStyle,
}

/// Create a horizontal level meter with gradient style (default).
pub fn level_meter(value: f64, min: f64, max: f64) -> LevelMeter {
    LevelMeter { value, min, max, orientation: Orientation::Horizontal, style: MeterStyle::Gradient }
}

impl LevelMeter {
    /// Set to vertical orientation.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Set the visual style (gradient or tint).
    pub fn style(mut self, style: MeterStyle) -> Self {
        self.style = style;
        self
    }

    /// Set tint mode: single solid color that transitions green → orange → red.
    pub fn tint(mut self) -> Self {
        self.style = MeterStyle::Tint;
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
        let w = LevelMeterWidget::new(self.value, self.min, self.max, self.orientation)
            .with_style(self.style);
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
        if prev.style != self.style {
            LevelMeterWidget::set_style(&mut element, self.style);
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
