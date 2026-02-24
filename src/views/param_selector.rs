//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx};

pub use crate::widgets::param_selector::LabelAlign;
use crate::widgets::param_selector::ParamSelector as SelectorWidget;

/// A vertical parameter selector view with text labels and dot indicator.
pub struct ParamSelector<F> {
    labels: Vec<String>,
    selected: usize,
    on_change: F,
    label_align: LabelAlign,
    tint: Option<xilem::masonry::vello::peniko::Color>,
}

/// Create a parameter selector with vertical text labels.
pub fn param_selector<State, Action>(
    labels: Vec<String>,
    selected: usize,
    on_change: impl Fn(&mut State, usize) -> Action + Send + Sync + 'static,
) -> ParamSelector<impl Fn(&mut State, usize) -> Action + Send + Sync + 'static> {
    ParamSelector {
        labels,
        selected,
        on_change,
        label_align: LabelAlign::Left,
        tint: None,
    }
}

impl<F> ParamSelector<F> {
    pub fn label_align(mut self, align: LabelAlign) -> Self {
        self.label_align = align;
        self
    }

    pub fn tint(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.tint = Some(color);
        self
    }
}

impl<F> ViewMarker for ParamSelector<F> {}

impl<F, State, Action> View<State, Action, ViewCtx> for ParamSelector<F>
where
    State: 'static,
    Action: 'static,
    F: Fn(&mut State, usize) -> Action + Send + Sync + 'static,
{
    type Element = Pod<SelectorWidget>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx, _: &mut State) -> (Self::Element, Self::ViewState) {
        let mut w = SelectorWidget::new(self.labels.clone(), self.selected, self.label_align);
        if let Some(c) = self.tint {
            w = w.with_tint(c);
        }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        _: &mut (),
        _: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        _: &mut State,
    ) {
        if prev.selected != self.selected {
            SelectorWidget::set_selected(&mut element, self.selected);
        }
        if prev.labels != self.labels {
            SelectorWidget::set_labels(&mut element, self.labels.clone());
        }
        if prev.tint != self.tint {
            if let Some(c) = self.tint {
                SelectorWidget::set_tint(&mut element, c);
            }
        }
    }

    fn teardown(&self, _: &mut (), ctx: &mut ViewCtx, element: Mut<'_, Self::Element>) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        _: &mut (),
        message: &mut MessageContext,
        _: Mut<'_, Self::Element>,
        state: &mut State,
    ) -> MessageResult<Action> {
        if message.take_first().is_some() {
            return MessageResult::Stale;
        }
        match message.take_message::<usize>() {
            Some(idx) => MessageResult::Action((self.on_change)(state, *idx)),
            None => MessageResult::Stale,
        }
    }
}
