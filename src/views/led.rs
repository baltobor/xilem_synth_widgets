//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageCtx, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Color, Pod, ViewCtx};

use crate::widgets::led::Led as LedWidget;

/// A small LED indicator light — a filled circle that shows on/off state.
///
/// Used for beat position indicators, status displays, etc.
/// The LED is display-only (no user interaction), just shows a colored circle.
pub struct Led {
    active: bool,
    tint: Option<Color>,
}

/// Create an LED indicator.
///
/// `active` controls whether the LED is lit (colored) or dim (dark gray).
pub fn led(active: bool) -> Led {
    Led { active, tint: None }
}

impl Led {
    /// Set the color of the LED when active. Default is orange (DEFAULT_TINT).
    pub fn tint(mut self, color: Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl ViewMarker for Led {}

impl<State, Action> View<State, Action, ViewCtx> for Led
where
    State: 'static,
    Action: 'static,
{
    type Element = Pod<LedWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = LedWidget::new(self.active);
        if let Some(c) = self.tint { w = w.with_tint(c); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self, prev: &Self, _: &mut (), _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>, _: &mut State,
    ) {
        if prev.active != self.active { LedWidget::set_active(&mut element, self.active); }
        if prev.tint != self.tint {
            if let Some(c) = self.tint { LedWidget::set_tint(&mut element, c); }
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
