#!/bin/bash
cargo build --release
./target/release/team >> release.log &
