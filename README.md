# Vonal global search

Vonal is a modern alternative to `dmenu`, `rofi`, `albert` etc...
Ideal for tiling window managers.

Currently only X11 is supported. (If you need Wayland support, please tell me.)

## Installation & Usage

Requirements: Rust.

1. Build: `cargo build --release`
2. Start the daemon: `./target/release/vonal`
3. Show the window: `./target/release/vonalc toggle`

If you use bspwm, an example rule to keep Vonal floating:
`bspc rule -a vonal state=floating border=off`

## The current state

Currently, there are 2 plugins:

- **Application launcher plugin:** A fuzzy search for .desktop files and executables in $PATH, supporting sub-actions
  - trigger: anything
  - shortcuts:
    - Up, Down, Left, Right, Enter
  - example commands:
    - `chr` finds chromium
    - `chr github.com` finds chromium and on enter, it opens it with `github.com`
    - `,` is for settings like reload application cache
- **Calculator plugin:** a python proxy
  - trigger: `=`
  - example commands:
    - `= sin(radians(90))` prints `1.0`
    - `= [i for i in range(1000) if i %99 == 0]` shows the numbers between 0 and 999 that are dividable by 99

## Contribution

Please let me know if you would like to use it. It would motivate me to work on the project.
Open any issues about new ideas. Tell me what plugins would you want to see.

Develop your own plugins, it's easy!
I put serious effort to make plugins both unlimited and simple.

The GUI is done with [egui](https://github.com/emilk/egui), which is tailored for quick progress.

An example plugin:
```rust

struct SayHiPlugin {}

impl Plugin for SayHiPlugin {
    fn search(
        &mut self,
        query: &mut String,
        ui: &mut Ui,
        _window: &GlutinWindowContext,
    ) -> PluginFlowControl {
      if query.starts_with("hello") {
        ui.label("Hi!");

        return PluginFlowControl::Break
      }


      PluginFlowControl::Continue
    }
}

```
