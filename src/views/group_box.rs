//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use xilem::core::{MessageContext, Mut, View, ViewMarker, ViewId, ViewPathTracker};
use xilem::core::MessageResult;
use xilem::{Pod, ViewCtx, WidgetView};

use crate::widgets::group_box::GroupBox as GroupBoxWidget;

const CHILD_VIEW_ID: ViewId = ViewId::new(0);

/// A group box view with a label and solid background.
pub struct GroupBox<V> {
    label: String,
    child: V,
    bg_color: Option<xilem::masonry::vello::peniko::Color>,
    tint: Option<xilem::masonry::vello::peniko::Color>,
    fill: bool,
}

/// Create a group box with a label and child content.
pub fn group_box<State, Action, V: WidgetView<State, Action>>(
    label: impl Into<String>,
    child: V,
) -> GroupBox<V> {
    GroupBox {
        label: label.into(),
        child,
        bg_color: None,
        tint: None,
        fill: false,
    }
}

impl<V> GroupBox<V> {
    pub fn bg_color(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    pub fn tint(mut self, color: xilem::masonry::vello::peniko::Color) -> Self {
        self.tint = Some(color);
        self
    }

    pub fn fill(mut self) -> Self {
        self.fill = true;
        self
    }
}

impl<V> ViewMarker for GroupBox<V> {}

impl<V, State, Action> View<State, Action, ViewCtx> for GroupBox<V>
where
    V: WidgetView<State, Action>,
    State: 'static,
    Action: 'static,
{
    type Element = Pod<GroupBoxWidget>;
    type ViewState = V::ViewState;

    fn build(
        &self,
        ctx: &mut ViewCtx,
        app_state: &mut State,
    ) -> (Self::Element, Self::ViewState) {
        let (child_pod, child_state) = ctx.with_id(CHILD_VIEW_ID, |ctx| {
            self.child.build(ctx, app_state)
        });
        let mut w = GroupBoxWidget::new(&self.label, child_pod.new_widget);
        if let Some(c) = self.bg_color { w = w.with_bg_color(c); }
        if let Some(c) = self.tint { w = w.with_tint(c); }
        if self.fill { w = w.with_fill(true); }
        let pod = ctx.with_action_widget(|ctx| ctx.create_pod(w));
        (pod, child_state)
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        if prev.label != self.label {
            GroupBoxWidget::set_label(&mut element, &self.label);
        }
        if prev.bg_color != self.bg_color {
            if let Some(c) = self.bg_color { GroupBoxWidget::set_bg_color(&mut element, c); }
        }
        if prev.tint != self.tint {
            if let Some(c) = self.tint { GroupBoxWidget::set_tint(&mut element, c); }
        }
        if prev.fill != self.fill {
            GroupBoxWidget::set_fill(&mut element, self.fill);
        }
        ctx.with_id(CHILD_VIEW_ID, |ctx| {
            self.child.rebuild(
                &prev.child,
                view_state,
                ctx,
                GroupBoxWidget::child_mut(&mut element).downcast(),
                app_state,
            );
        });
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        mut element: Mut<'_, Self::Element>,
    ) {
        ctx.with_id(CHILD_VIEW_ID, |ctx| {
            self.child.teardown(
                view_state,
                ctx,
                GroupBoxWidget::child_mut(&mut element).downcast(),
            );
        });
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut MessageContext,
        mut element: Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match message.take_first() {
            Some(CHILD_VIEW_ID) => self.child.message(
                view_state,
                message,
                GroupBoxWidget::child_mut(&mut element).downcast(),
                app_state,
            ),
            _ => MessageResult::Stale,
        }
    }
}
