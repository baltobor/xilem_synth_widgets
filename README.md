# Xilem Synth Widgets

Synthesizer-style UI widgets for [Xilem](https://github.com/linebender/xilem): knobs, faders, oscilloscope, and more.

A collection of audio/synth-themed controls designed for music applications, built on [Xilem](https://github.com/linebender/xilem) 0.4.0

<img width="802" height="385" alt="XilemSynthWidgetsArch" src="https://github.com/user-attachments/assets/fc890b15-2c6d-4ad0-8915-830a8678edad" />

This screenshot was taken on Arch linux

Tested on 
- MacOS Tahoe 26.3
- Arch linux and Cosmic

## Run demo

```shell
cargo run --example demo
```

## Usage

```rust
use xilem_synth_widgets::{
    knob, fader, param_selector, push_button, group_box, scope,
    LabelAlign, ScopeSource,
};
```

## Widgets

### Knob
Rotary control for continuous parameters. Drag vertically to adjust. Double-click resets to default. The lit arc shows distance from the default value.

`knob(min, max, value, default, on_change)`

```rust
knob(0.0, 100.0, state.value, 50.0, |s: &mut State, v| s.value = v)
    .step(1.0)      // quantize to steps
    .small()        // smaller variant
    .tint(color)    // custom accent color
```

### Fader
Vertical slider with logarithmic dB scaling. Drag to adjust, double-click resets.

`fader(min_db, max_db, value_db, default_db, on_change)`

```rust
fader(-60.0, 6.0, state.volume_db, -12.0, |s: &mut State, v| s.volume_db = v)
    .tint(color)
```

### Param Selector
Vertical list for discrete options. Click to select.

`param_selector(labels, selected, on_change)`

```rust
param_selector(
    vec!["Sine".into(), "Saw".into(), "Triangle".into()],
    state.waveform,
    |s: &mut State, idx| s.waveform = idx,
)
.label_align(LabelAlign::Right)
.tint(color)
```

### Push Button
Small circular toggle for boolean options.

`push_button(active, on_toggle)`

```rust
push_button(state.enabled, |s: &mut State, v| s.enabled = v)
    .tint(color)
```

### Group Box
Labeled container for grouping controls.

`group_box(label, child)`

```rust
group_box("Oscillator", flex_col((
    knob(...),
    label("Freq"),
)))
.tint(IVORY)
.fill()
```

### Scope
Real-time oscilloscope display. Accepts a `ScopeSource` for lock-free audio data from your DSP thread.

`scope(source)` where `source: Option<ScopeSource>`

```rust
// Create a ScopeSource from a triple_buffer::Output<Vec<f32>>
let source = ScopeSource::new(triple_buffer_output);

scope(Some(source))
    .wave_color(Color::from_rgb8(0x00, 0xFF, 0x80))
```

## Example

See `examples/demo.rs` for a complete synthesizer demo with audio output.

```sh
cargo run --example demo
```

## Motivation

I tried to keep the code as compact and clean as possible. Not only to make it readable, but above all to show how simple it is to program beautiful UIs with Xilem. The framework is still very young, but I am already convinced that it allows you to work very quickly and build great UIs.
And now I am looking forward to seeing lots of beautiful applications. ;-) 

Thank you very much, Xilem team! I've been waiting for this for so long! And now it's there.

## License

MIT License - see LICENSE file.

Compatible with Xilem's Apache 2.0 license.

**This software is provided "as is", without warranty of any kind.**

## Author

Jacek Wisniowski

## Acknowledgments

Thanks to the [linebender](https://github.com/linebender) team for creating Xilem and the surrounding ecosystem. Special thanks to Olivier Faure for the encouragement and support.
