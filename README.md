# Vonal global search

Vonal is a modern alternative to `dmenu`, `rofi`, `albert` etc... 
Ideal for tiling window managers.

## Installation & Usage

Requirements: Rust.

1. Build: `cargo build --release`
2. Start the daemon: `./target/release/vonal` 
3. Show the window: `./target/release/vonalc toggle` 

If you use bspwm, an example rule to keep Vonal floating:
`bspc rule -a vonal state=floating border=off`

## The current state

Currently, there are 2 plugins:
 - **Application launcher plugin:** A fuzzy search for .desktop files, supporting sub-actions
    - trigger: anything
    - shortcuts:
        - Up, Down, Left, Right, Enter
    - example commands:
        - `chr` will find chromium
        - `,` is for settings like reload application cache
 - **Calculator plugin:** a python proxy
    - trigger: `=`
    - example commands:
        - `= sin(radians(90))` will show the result of `1.0`
        - `= [i for i in range(1000) if i %99 == 0]` will show the numbers between 0 and 999 that are dividable by 99
