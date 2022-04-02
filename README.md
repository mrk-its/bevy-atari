# bevy-atari - Good Enough Atari XL/XE Emulator

It is written from scratch in [Rust](https://www.rust-lang.org/) on the top of great multiplatform [Bevy Game Engine](https://github.com/bevyengine/bevy)
It is still far from perfect (and won't compete with Alirra for emulation quality), but already good enough to run a lot of Atari software directly in the browser window.


## Features
* Cross-platform - primary target is wasm32 running in the browser, but native executables for Linux / Windows / MacOSX can also be build.
* No pre-configuration required, images configured via URL parameters (it uses CORS proxy to be able to download images from external services)
* ANTIC / GTIA is emulated on GPU (using this [fragment shader](https://github.com/mrk-its/bevy-atari-antic/blob/main/src/render/antic.wgsl)), reducing CPU usage of single browser thread. Requires WebGL2 in the browser.
* Cycle-accurate 6502 emulation using [emulator_6502](https://github.com/GarettCooper/emulator_6502), with invalid opcodes and proper DMA cycle stealing.
* Very good POKEY emulation (including stereo) with [Web-Pokey](https://github.com/mrk-its/web-pokey)
* 256 kB extended memory by default.
* ATR disk image support
* CAR cartrige image support (currently Standard 8k / AtariMax 128k / AtariMax 1M, more will be added if required)
* GamePad support with Gamepad API

## Known Limitations
* Simplified ANTIC / GTIA emulation - mid-screen registry changes are not visible on the screen instantly
* POKEY interrupts are not supported yet.
* no SIO emulation yet (for now IO is done by SIO patch)
* no casette image emulation.

There are also tons of other bugs, causing screen glitches or simply crashing emulated programs. If you find any, or if you simply have a feature request, please fill an [issue](https://github.com/mrk-its/bevy-atari/issues)

Few live games:
* [Avalon Robbo (demo)](https://mrk.sed.pl/bevy-atari/#https://atarionline.pl/arch/R/Robbo%20(L.K.%20Avalon)/Robbo%20(demo)%20(1989)(L.K.%20Avalon)(PL).xex)
* [FloB](https://mrk.sed.pl/bevy-atari/#https://bocianu.atari.pl/assets/games/flob.1.0.3b.car)
* [Gacek (ABBUC 2021)](https://mrk.sed.pl/bevy-atari/#xex=https://atarionline.pl/forum/?PostBackAction=Download&AttachmentID=18196)
* [Last Squadron (ABBUC 2021 version)](https://mrk.sed.pl/bevy-atari/#disk_1=https://atarionline.pl/forum/?PostBackAction=Download&AttachmentID=15974)
* [Prince of Persia](https://mrk.sed.pl/bevy-atari/#https://atari.ha.sed.pl/pop.car)

And, as a bonus, multi emulation example:
* [Atari Wall](https://mrk.sed.pl/bevy-atari/multi/#xex@0=https://atarionline.pl/demoscena/R/Revenge%20of%20Magnus.xex&&xex@1=https://atarionline.pl/demoscena/L/Laser%20Demo.xex&&car@2=https://atari.ha.sed.pl/pop.car&&xex@3=https://atarionline.pl/demoscena/F/Five%20to%20Five.xex&&disk_1@5=https://atarionline.pl/demoscena/D/Drunk%20Chessboard.atr&&xex@4=https://atarionline.pl/demoscena/cp/Silly%20Venture%202010/Control.xex&&disk_1@6=https://atarionline.pl/demoscena/A/Asskicker,%20The%20(128,v2).atr&&disk_1@7=https://atarionline.pl/demoscena/I/Isolation%20(128,v2).atr&&disk_1@8=https://atari.ha.sed.pl/ferris.xex)

## Build instructions

### Prerequisites
Install Rust: https://www.rust-lang.org/learn/get-started, then:
```
cargo install cargo-make
```
```
cargo make serve
```
and point your browser [here](http://127.0.0.1:4000/).