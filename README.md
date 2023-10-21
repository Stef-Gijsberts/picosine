# PicoSine

This repository contains a basic synthesizer audio plugin. I work on this to
learn about CLAP (CLever Audio Plug-in API).

## Building

1. [Install Rust](https://www.rust-lang.org/tools/install).
2. Clone this repository and navigate to it.
3. Run `cargo build` to build the plugin.

## Trying

Copy the built plugin to a local CLAP plugin folder. For example, on Linux:

```sh
cp ./target/debug/libpicosine.so ~/.clap/picosine.clap
```

Then use a DAW that supports CLAP plugins (I use Bitwig), and load the plugin.

## About CLAP

CLAP stands for CLever Audio Plugin.

Sources:

- Website with general background, good starting point: https://cleveraudio.org/
- The git repository for the API: https://github.com/free-audio/clap
- Clack, a safe Rust wrapper for the CLAP API: https://github.com/prokopyl/clack

## Copying

    PicoSine - a basic synthesizer audio plugin.
    Copyright (C) 2023  Stef Gijsberts

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <http://www.gnu.org/licenses/>.
