set export := true
set ignore-comments := true
set positional-arguments := true
set shell := ["/usr/bin/env", "bash", "-euo", "pipefail", "-c"]

_just := quote(just_executable()) + " --justfile=" + quote(justfile())
_setup_bash := "set -euo pipefail"
CARGO := "cargo"

_default:
    @command {{ _just }} --list

check:
    $CARGO clippy --all
    $CARGO clippy --all --features string

fmt:
    $CARGO fmt
