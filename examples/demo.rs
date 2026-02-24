//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).

use std::sync::Arc;

use xilem::masonry::properties::types::{AsUnit, CrossAxisAlignment};
use xilem::masonry::vello::peniko::Color;
use xilem::style::Style;
use xilem::view::{flex_col, flex_row, label, FlexExt as _, FlexSpacer};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

mod dsp;
use dsp::{list_devices, DspEngine, DspHandle, SharedParams};

use xilem_synth_widgets::{
    fader, group_box, knob, param_selector, push_button, scope, LabelAlign,
};

const TEXT_COLOR: Color = Color::from_rgb8(0xDD, 0xCC, 0xCC);
const DIM_TEXT: Color = Color::from_rgb8(0xAA, 0x99, 0x99);
const IVORY: Color = Color::from_rgb8(0xF8, 0xF1, 0xE5);

struct DemoState {
    freq1: f64,
    freq2: f64,
    volume_db: f64,
    waveform: usize,
    lfo_enabled: bool,
    lfo_range: f64,
    lfo_speed: f64,
    mute: bool,
    devices: Vec<String>,
    selected_device: usize,
    audio_started: bool,
    dsp: DspHandle,
}

impl DemoState {
    fn new() -> Self {
        let params = Arc::new(SharedParams::new(220.0, 330.0, -12.0, true));
        let devices = list_devices();
        let dsp = DspHandle::new_idle(Arc::clone(&params));
        Self {
            freq1: 220.0,
            freq2: 330.0,
            volume_db: -12.0,
            waveform: 0,
            lfo_enabled: true,
            lfo_range: 8.0,
            lfo_speed: 0.0001,
            mute: false,
            devices,
            selected_device: 0,
            audio_started: false,
            dsp,
        }
    }
}

fn app_logic(state: &mut DemoState) -> impl WidgetView<DemoState> + use<> {
    let db_text = if state.volume_db <= -60.0 {
        "-inf dB".to_string()
    } else {
        format!("{:.1} dB", state.volume_db)
    };

    let device_names: Vec<String> = if state.devices.is_empty() {
        vec!["(no devices)".into()]
    } else {
        state.devices.clone()
    };

    // Create the Xilem GUI!
    // Thanks to Olivier Faure for picking me up on github.
    group_box(
        "Synth Widget Demo",
        flex_col((
            // Top bar - audio device selection
            group_box(
                "Audio",
                flex_col((
                    flex_row((
                        param_selector(
                            device_names,
                            state.selected_device,
                            |s: &mut DemoState, idx| {
                                s.selected_device = idx;
                                if s.audio_started {
                                    s.dsp.stop();
                                    let device_name = s.devices.get(idx).map(|n| n.as_str());
                                    match DspEngine::start(device_name, Arc::clone(&s.dsp.params))
                                    {
                                        Ok(handle) => s.dsp = handle,
                                        Err(e) => {
                                            eprintln!("Failed to restart audio: {e}");
                                            s.audio_started = false;
                                        }
                                    }
                                }
                            },
                        )
                        .label_align(LabelAlign::Right),
                        push_button(state.audio_started, |s: &mut DemoState, v| {
                            if v {
                                let device_name =
                                    s.devices.get(s.selected_device).map(|n| n.as_str());
                                match DspEngine::start(device_name, Arc::clone(&s.dsp.params)) {
                                    Ok(handle) => {
                                        s.dsp = handle;
                                        s.audio_started = true;
                                    }
                                    Err(e) => {
                                        eprintln!("Failed to start audio: {e}");
                                        s.audio_started = false;
                                    }
                                }
                            } else {
                                s.dsp.stop();
                                s.audio_started = false;
                            }
                        }),
                        label("Start").text_size(10.0).color(DIM_TEXT),
                        FlexSpacer::Flex(1.0),
                    ))
                    .gap(6.0.px()),
                    FlexSpacer::Fixed(4.0.px()),
                    label("Developed in Rust. Based on linebender's Xilem UI.")
                        .text_size(9.0)
                        .color(DIM_TEXT),
                    label("Develop gorgeous applications.")
                        .text_size(9.0)
                        .color(DIM_TEXT),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(2.0.px()),
            ),
            // Main controls row
            flex_row((
                // Oscillators group
                group_box(
                    "Oscillators",
                    flex_col((
                        flex_row((
                            param_selector(
                                vec![
                                    "Sine".into(),
                                    "Saw".into(),
                                    "Tri".into(),
                                    "Pulse".into(),
                                ],
                                state.waveform,
                                |s: &mut DemoState, idx| {
                                    s.waveform = idx;
                                    s.dsp.params.set_waveform(idx as u32);
                                },
                            )
                            .label_align(LabelAlign::Left),
                            flex_col((
                                label(format!("{:.0} Hz", state.freq1))
                                    .text_size(11.0)
                                    .color(TEXT_COLOR),
                                knob(
                                    20.0,
                                    2000.0,
                                    state.freq1,
                                    220.0,
                                    |s: &mut DemoState, v| {
                                        s.freq1 = v;
                                        s.dsp.params.freq1.store(v as f32);
                                    },
                                )
                                .step(1.0),
                                label("Freq 1").text_size(10.0).color(DIM_TEXT),
                            ))
                            .gap(1.0.px()),
                            flex_col((
                                label(format!("{:.0} Hz", state.freq2))
                                    .text_size(11.0)
                                    .color(TEXT_COLOR),
                                knob(
                                    20.0,
                                    2000.0,
                                    state.freq2,
                                    330.0,
                                    |s: &mut DemoState, v| {
                                        s.freq2 = v;
                                        s.dsp.params.freq2.store(v as f32);
                                    },
                                )
                                .step(1.0),
                                label("Freq 2").text_size(10.0).color(DIM_TEXT),
                            ))
                            .gap(1.0.px()),
                        ))
                        .gap(6.0.px()),
                        flex_row((
                            flex_col((
                                label(format!("{:.0} Hz", state.lfo_range))
                                    .text_size(9.0)
                                    .color(TEXT_COLOR),
                                knob(
                                    2.0,
                                    20.0,
                                    state.lfo_range,
                                    8.0,
                                    |s: &mut DemoState, v| {
                                        s.lfo_range = v;
                                        s.dsp.params.lfo_range.store(v as f32);
                                    },
                                )
                                .step(0.5)
                                .small(),
                                label("Range").text_size(9.0).color(DIM_TEXT),
                            ))
                            .gap(1.0.px()),
                            flex_col((
                                label(format!("{:.4}", state.lfo_speed))
                                    .text_size(9.0)
                                    .color(TEXT_COLOR),
                                knob(
                                    0.00001,
                                    0.001,
                                    state.lfo_speed,
                                    0.0001,
                                    |s: &mut DemoState, v| {
                                        s.lfo_speed = v;
                                        s.dsp.params.lfo_speed.store(v as f32);
                                    },
                                )
                                .small(),
                                label("Speed").text_size(9.0).color(DIM_TEXT),
                            ))
                            .gap(1.0.px()),
                            flex_col((
                                push_button(state.lfo_enabled, |s: &mut DemoState, v| {
                                    s.lfo_enabled = v;
                                    s.dsp.params.set_lfo_enabled(v);
                                }),
                                label("LFO").text_size(9.0).color(DIM_TEXT),
                            ))
                            .gap(1.0.px()),
                        ))
                        .gap(6.0.px()),
                    ))
                    .gap(4.0.px()),
                ),
                // Output fader
                group_box(
                    "Output",
                    flex_col((
                        label(db_text).text_size(11.0).color(TEXT_COLOR),
                        fader(-60.0, 6.0, state.volume_db, -12.0, |s: &mut DemoState, v| {
                            s.volume_db = v;
                            s.dsp.params.volume_db.store(v as f32);
                        }),
                        label("Volume").text_size(10.0).color(DIM_TEXT),
                        push_button(state.mute, |s: &mut DemoState, v| {
                            s.mute = v;
                            s.dsp.params.set_mute(v);
                        }),
                        label("Mute").text_size(9.0).color(DIM_TEXT),
                    ))
                    .gap(2.0.px()),
                ),
                // Scope
                group_box::<DemoState, (), _>(
                    "Scope",
                    scope(Some(state.dsp.scope_source())),
                ),
                // Info
                group_box::<DemoState, (), _>(
                    "Info",
                    flex_col((
                        label("Knobs").text_size(11.0).color(TEXT_COLOR),
                        label("  Drag up/down to adjust").text_size(10.0).color(DIM_TEXT),
                        label("  Double-click resets to default").text_size(10.0).color(DIM_TEXT),
                        label("  Lit ring shows delta from default").text_size(10.0).color(DIM_TEXT),
                        label("Fader").text_size(11.0).color(TEXT_COLOR),
                        label("  Drag to adjust, defaults to -12 dB").text_size(10.0).color(DIM_TEXT),
                        label("  Double-click resets to default").text_size(10.0).color(DIM_TEXT),
                        label("Selector").text_size(11.0).color(TEXT_COLOR),
                        label("  Click item to select").text_size(10.0).color(DIM_TEXT),
                        label("Button").text_size(11.0).color(TEXT_COLOR),
                        label("  Click to toggle on/off").text_size(10.0).color(DIM_TEXT),
                        label("Group Box").text_size(11.0).color(TEXT_COLOR),
                        label("  Tintable background container").text_size(10.0).color(DIM_TEXT),
                    ))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(1.0.px()),
                ).fill().flex(1.0),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Fill)
            .must_fill_major_axis(true)
            .gap(4.0.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .gap(4.0.px()),
    )
    .tint(IVORY)
}

/// Create the Xilem app
fn main() {

    // If you were looking for something complicated,
    // I'm afraid I'll have to disappoint you.
    // Super clean code, isn't it?
    let app = Xilem::new_simple(
        DemoState::new(),
        app_logic,
        WindowOptions::new("Synth Widget Demo")
            .with_initial_inner_size(xilem::winit::dpi::LogicalSize::new(802.0, 385.0)),
    );
    app.run_in(EventLoop::with_user_event()).unwrap();
}
