//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

mod widgets;
mod views;
pub mod theme;

pub use views::fader::fader;
pub use views::group_box::group_box;
pub use views::knob::knob;
pub use views::param_selector::{param_selector, LabelAlign};
pub use views::push_button::push_button;
pub use views::scope::{scope, ScopeBuffer, ScopeSource};

pub use xilem;
