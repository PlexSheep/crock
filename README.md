# Crock

![Project badge](https://img.shields.io/badge/language-Rust-blue.svg)
![Crates.io License](https://img.shields.io/crates/l/crock)
![Gitea Release](https://img.shields.io/gitea/v/release/PlexSheep/crock?gitea_url=https%3A%2F%2Fgit.cscherr.de)
![Gitea language count](https://img.shields.io/gitea/languages/count/PlexSheep/crock?gitea_url=https%3A%2F%2Fgit.cscherr.de)
[![cargo checks and tests](https://github.com/PlexSheep/crock/actions/workflows/cargo.yaml/badge.svg)](https://github.com/PlexSheep/crock/actions/workflows/cargo.yaml)

A little clock for your terminal, written in rust.

![screenshot](data/media/screenshot.png)

* [Original Repository](https://git.cscherr.de/PlexSheep/crock)
* [GitHub Mirror](https://github.com/PlexSheep/crock)
* [crates.io](https://crates.io/crates/crock)

## Compilation

The `desktop` and `sound` features require additional system dependencies:

| Feature   | Dependency | PKG Name on Debian based Distributions |
|-----------|------------|----------------------------------------|
| `desktop` | dbus       | `libdbus-1-dev`                        |
| `sound`   | alsa       | `libasound2-dev`                       |

If you want to compile without these features, you will not have notifications 
and sound alerts for countdown mode. (Use `cargo build -r --no-default-features`)

## Acknoledgements

The included alarm sound is from [freesound.org](https://freesound.org):

-> ["effect_notify.wav" by ricemaster (CC-0)](https://freesound.org/people/ricemaster/sounds/278142/)
