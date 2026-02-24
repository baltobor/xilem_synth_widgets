//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

use crate::widgets::push_button::PushButton as ButtonWidget;

/// A small circular toggle button view for boolean options.
pub struct PushButton<F> {
    active: bool,
    on_toggle: F,
    tint: Option<xilem::masonry::vello::peniko::Color>,
}

/// Create a push button (boolean toggle).
pub fn push_button<State, Action>(
    active: bool,
    on_toggle: impl Fn(&mut State, bool) -> Action + Send + Sync + 'static,
) -> PushButton<impl Fn(&mut State, bool) -> Action + Send + Sync + 'static> {
    PushButton { active, on_toggle, tint: None }
}

impl<F> PushButton<F> {
    pub fn tint(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl<F> ViewMarker for PushButton<F> {}

impl<F, State, Action> View<State, Action, ViewCtx> for PushButton<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, bool) -> Action + Send + Sync + 'static,
{
    type Element = Pod<ButtonWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = ButtonWidget::new(self.active);
        if let Some(c) = self.tint { w = w.with_tint(c); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self, prev: &Self, _: &mut (), _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>, _: &mut State,
    ) {
        if prev.active != self.active { ButtonWidget::set_active(&mut element, self.active); }
        if prev.tint != self.tint {
            if let Some(c) = self.tint { ButtonWidget::set_tint(&mut element, c); }
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
        match message.take_message::<bool>() {
            Some(val) => MessageResult::Action((self.on_toggle)(state, *val)),
            None => MessageResult::Stale,
        }
    }
}
