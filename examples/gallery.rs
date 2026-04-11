//! This file is part of the xilem_synth_widgets project.
//! (c) 2026 by Jacek Wisniowski
//!
//! This project was released as open source under the
//! Apache License, Version 2.0: http://www.apache.org/licenses/LICENSE-2.0
//! (compatible with the Xilem licence).
//!
//! Widget Gallery — demonstrates every widget and its styling options.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use std::f64::consts::TAU;

use xilem::tokio::time;

use xilem::core::fork;
use xilem::masonry::layout::AsUnit;
use xilem::masonry::properties::types::CrossAxisAlignment;
use xilem::Color;
use xilem::style::Style as _;
use xilem::view::{flex_col, flex_row, label, task, FlexExt as _, FlexSpacer};
use xilem::{EventLoop, WidgetView, WindowOptions, Xilem};

use xilem_synth_widgets::{
    fader, group_box, knob, led, level_meter, param_selector, push_button, scope,
    LabelAlign, ScopeSource,
};

// ── colors ──────────────────────────────────────────────────────────────

const TEXT: Color = Color::from_rgb8(0xDD, 0xCC, 0xCC);
const DIM: Color = Color::from_rgb8(0xAA, 0x99, 0x99);

// GroupBox palette
const IVORY: Color = Color::from_rgb8(0xF8, 0xF1, 0xE5);
const SLATE: Color = Color::from_rgb8(0x3A, 0x4A, 0x5A);
const CHARCOAL: Color = Color::from_rgb8(0x2A, 0x2A, 0x2A);
const TEAL: Color = Color::from_rgb8(0x20, 0x80, 0x80);
const BURGUNDY: Color = Color::from_rgb8(0x80, 0x20, 0x30);
const OLIVE: Color = Color::from_rgb8(0x55, 0x66, 0x33);
const SAND: Color = Color::from_rgb8(0xD2, 0xB4, 0x8C);
const LAVENDER: Color = Color::from_rgb8(0xB0, 0xA0, 0xD0);
const ROSE: Color = Color::from_rgb8(0xE8, 0xA0, 0xB0);
const SKY: Color = Color::from_rgb8(0x70, 0xC0, 0xE8);

// ── state ───────────────────────────────────────────────────────────────

struct GalleryState {
    phase: f64,
    scope_source: ScopeSource,
    _anim_running: Arc<AtomicBool>,

    // Interactive widget state
    knob_a: f64,
    knob_b: f64,
    knob_c: f64,
    fader_val: f64,
    selector_idx: usize,
    bool_idx: usize,
    push_a: bool,
    push_b: bool,
    push_c: bool,
}

impl GalleryState {
    fn new() -> Self {
        let (scope_input, scope_output) = triple_buffer::triple_buffer(&vec![0.0f32; 1024]);
        let scope_source = ScopeSource::new(scope_output);
        let running = Arc::new(AtomicBool::new(true));

        // Background thread generates a sine wave into the scope
        let running_clone = Arc::clone(&running);
        let mut input = scope_input;
        std::thread::spawn(move || {
            let sample_rate = 44100.0_f64;
            let freq = 220.0;
            let buf_size = 1024;
            let mut phase = 0.0_f64;
            let phase_inc = freq / sample_rate;

            while running_clone.load(Ordering::Relaxed) {
                let mut buf = vec![0.0f32; buf_size];
                for s in buf.iter_mut() {
                    *s = (phase * TAU).sin() as f32;
                    phase += phase_inc;
                    if phase >= 1.0 {
                        phase -= 1.0;
                    }
                }
                input.write(buf);
                std::thread::sleep(Duration::from_millis(16));
            }
        });

        Self {
            phase: 0.0,
            scope_source,
            _anim_running: running,
            knob_a: 0.5,
            knob_b: 220.0,
            knob_c: 0.75,
            fader_val: -12.0,
            selector_idx: 0,
            bool_idx: 0,
            push_a: false,
            push_b: true,
            push_c: false,
        }
    }
}

// ── helpers ─────────────────────────────────────────────────────────────

/// Simple sine-based animation value (0..1) at the given frequency.
fn anim_sin(phase: f64, freq: f64) -> f64 {
    ((phase * freq * TAU).sin() * 0.5 + 0.5).clamp(0.0, 1.0)
}

/// Map 0..1 to a dB range.
fn to_db(norm: f64, min_db: f64, max_db: f64) -> f64 {
    min_db + norm * (max_db - min_db)
}

// ── UI ──────────────────────────────────────────────────────────────────

fn app_logic(state: &mut GalleryState) -> impl WidgetView<GalleryState> + use<> {
    // Animated meter values
    let meter_a = to_db(anim_sin(state.phase, 0.7), -60.0, 6.0);
    let meter_b = to_db(anim_sin(state.phase, 1.1), -60.0, 6.0);
    let meter_c = to_db(anim_sin(state.phase, 0.3), -60.0, 6.0);
    let meter_d = to_db(anim_sin(state.phase, 0.5), -60.0, 6.0);

    // Animated LED states
    let led_phase = (state.phase * 2.0) % 4.0;

    fork(
        group_box(
            "Widget Gallery",
            flex_col((
                // ─── Row 1: GroupBox color showcase ─────────────────
                group_box(
                    "GroupBox Colors (automatic header contrast colors)",
                    flex_row((
                        group_box::<GalleryState, (), _>(
                            "Ivory",
                            label("bright").text_size(9.0).color(DIM),
                        ).tint(IVORY).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Sand",
                            label("creme").text_size(9.0).color(DIM),
                        ).tint(SAND).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Rose",
                            label("bright").text_size(9.0).color(DIM),
                        ).tint(ROSE).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Lavender",
                            label("bright").text_size(9.0).color(DIM),
                        ).tint(LAVENDER).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Sky",
                            label("bright").text_size(9.0).color(DIM),
                        ).tint(SKY).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Teal",
                            label("dark").text_size(9.0).color(DIM),
                        ).tint(TEAL).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Slate",
                            label("dark").text_size(9.0).color(DIM),
                        ).tint(SLATE).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Charcoal",
                            label("dark").text_size(9.0).color(DIM),
                        ).tint(CHARCOAL).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Burgundy",
                            label("dark").text_size(9.0).color(DIM),
                        ).tint(BURGUNDY).fill().flex(1.0),
                        group_box::<GalleryState, (), _>(
                            "Olive",
                            label("dark").text_size(9.0).color(DIM),
                        ).tint(OLIVE).fill().flex(1.0),
                    ))
                    .gap(4.0.px()),
                ),

                // ─── Row 2: Interactive widgets ─────────────────────
                flex_row((
                    // Knobs
                    group_box(
                        "Knobs",
                        flex_row((
                            flex_col((
                                label(format!("{:.2}", state.knob_a))
                                    .text_size(10.0).color(TEXT),
                                knob(0.0, 1.0, state.knob_a, 0.5,
                                    |s: &mut GalleryState, v| { s.knob_a = v; }),
                                label("Normal").text_size(9.0).color(DIM),
                            )).gap(1.0.px()),
                            flex_col((
                                label(format!("{:.0} Hz", state.knob_b))
                                    .text_size(10.0).color(TEXT),
                                knob(20.0, 2000.0, state.knob_b, 220.0,
                                    |s: &mut GalleryState, v| { s.knob_b = v; })
                                    .step(1.0),
                                label("Stepped").text_size(9.0).color(DIM),
                            )).gap(1.0.px()),
                            flex_col((
                                label(format!("{:.2}", state.knob_c))
                                    .text_size(10.0).color(TEXT),
                                knob(0.0, 1.0, state.knob_c, 0.75,
                                    |s: &mut GalleryState, v| { s.knob_c = v; })
                                    .small()
                                    .tint(TEAL),
                                label("Small").text_size(9.0).color(DIM),
                            )).gap(1.0.px()),
                        ))
                        .gap(8.0.px()),
                    )
                    .tint(SLATE),

                    // Fader
                    group_box(
                        "Fader",
                        flex_col((
                            label(if state.fader_val <= -60.0 {
                                "-inf dB".into()
                            } else {
                                format!("{:.1} dB", state.fader_val)
                            }).text_size(10.0).color(TEXT),
                            fader(-60.0, 6.0, state.fader_val, -12.0,
                                |s: &mut GalleryState, v| { s.fader_val = v; })
                                .tint(TEAL),
                            label("Volume").text_size(9.0).color(DIM),
                        ))
                        .gap(2.0.px()),
                    )
                    .tint(CHARCOAL),

                    // Selector & Bool
                    group_box(
                        "Selectors",
                        flex_row((
                            flex_col((
                                param_selector(
                                    vec!["Sine".into(), "Saw".into(), "Tri".into(), "Pulse".into()],
                                    state.selector_idx,
                                    |s: &mut GalleryState, idx| { s.selector_idx = idx; },
                                ).label_align(LabelAlign::Left),
                                label("Waveform").text_size(9.0).color(DIM),
                            )).gap(2.0.px()),
                            flex_col((
                                param_selector(
                                    vec!["Off".into(), "On".into()],
                                    state.bool_idx,
                                    |s: &mut GalleryState, idx| { s.bool_idx = idx; },
                                ).label_align(LabelAlign::Right)
                                    .tint(TEAL),
                                label("Bool").text_size(9.0).color(DIM),
                            )).gap(2.0.px()),
                        ))
                        .gap(8.0.px()),
                    )
                    .tint(OLIVE),

                    // Push Buttons & LEDs
                    group_box(
                        "Buttons",
                        flex_col((
                            flex_row((
                                flex_col((
                                    push_button(state.push_a,
                                        |s: &mut GalleryState, v| { s.push_a = v; }),
                                    label("Default").text_size(9.0).color(DIM),
                                )).gap(2.0.px()),
                                flex_col((
                                    push_button(state.push_b,
                                        |s: &mut GalleryState, v| { s.push_b = v; })
                                        .tint(TEAL),
                                    label("Teal").text_size(9.0).color(DIM),
                                )).gap(2.0.px()),
                                flex_col((
                                    push_button(state.push_c,
                                        |s: &mut GalleryState, v| { s.push_c = v; })
                                        .tint(ROSE),
                                    label("Rose").text_size(9.0).color(DIM),
                                )).gap(2.0.px()),
                            ))
                            .gap(6.0.px()),
                            FlexSpacer::Fixed(4.0.px()),
                            // LEDs
                            flex_row((
                                led(led_phase < 1.0),
                                led(led_phase >= 1.0 && led_phase < 2.0).tint(TEAL),
                                led(led_phase >= 2.0 && led_phase < 3.0).tint(SKY),
                                led(led_phase >= 3.0).tint(ROSE),
                            ))
                            .gap(4.0.px()),
                            label("LEDs (animated)").text_size(9.0).color(DIM),
                        ))
                        .gap(2.0.px()),
                    )
                    .tint(BURGUNDY),

                    // Scope
                    group_box::<GalleryState, (), _>(
                        "Scope",
                        scope(Some(state.scope_source.clone())),
                    )
                    .tint(CHARCOAL),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(4.0.px()),

                // ─── Row 3: Level Meters ────────────────────────────
                flex_row((
                    group_box::<GalleryState, (), _>(
                        "Level Meters — Gradient",
                        flex_col((
                            flex_row((
                                label("H").text_size(9.0).color(DIM),
                                level_meter(meter_a, -60.0, 6.0),
                            )).gap(4.0.px()),
                            flex_row((
                                label("H").text_size(9.0).color(DIM),
                                level_meter(meter_b, -60.0, 6.0),
                            )).gap(4.0.px()),
                            FlexSpacer::Fixed(4.0.px()),
                            flex_row((
                                level_meter(meter_c, -60.0, 6.0).vertical(),
                                level_meter(meter_d, -60.0, 6.0).vertical(),
                            ))
                            .gap(4.0.px()),
                            label("Vertical").text_size(9.0).color(DIM),
                        ))
                        .gap(2.0.px()),
                    )
                    .tint(SLATE),

                    group_box::<GalleryState, (), _>(
                        "Level Meters — Tint",
                        flex_col((
                            flex_row((
                                label("H").text_size(9.0).color(DIM),
                                level_meter(meter_a, -60.0, 6.0).tint(),
                            )).gap(4.0.px()),
                            flex_row((
                                label("H").text_size(9.0).color(DIM),
                                level_meter(meter_b, -60.0, 6.0).tint(),
                            )).gap(4.0.px()),
                            FlexSpacer::Fixed(4.0.px()),
                            flex_row((
                                level_meter(meter_c, -60.0, 6.0).vertical().tint(),
                                level_meter(meter_d, -60.0, 6.0).vertical().tint(),
                            ))
                            .gap(4.0.px()),
                            label("Vertical").text_size(9.0).color(DIM),
                        ))
                        .gap(2.0.px()),
                    )
                    .tint(CHARCOAL),

                    FlexSpacer::Flex(1.0),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(4.0.px()),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(4.0.px()),
        )
        .tint(IVORY),
        // Animation timer: triggers view rebuilds at ~30fps
        task(
            |proxy, _| async move {
                let mut interval = time::interval(Duration::from_millis(33));
                loop {
                    interval.tick().await;
                    let Ok(()) = proxy.message(()) else { break; };
                }
            },
            |state: &mut GalleryState, ()| {
                state.phase += 1.0 / 30.0;
            },
        ),
    )
}

fn main() {
    let app = Xilem::new_simple(
        GalleryState::new(),
        app_logic,
        WindowOptions::new("Widget Gallery")
            .with_initial_inner_size(xilem::winit::dpi::LogicalSize::new(900.0, 520.0)),
    );
    app.run_in(EventLoop::with_user_event()).unwrap();
}
