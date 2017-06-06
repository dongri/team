#!/bin/bash
cargo build --release
kill -9 $(lsof -ti tcp:3000)
./target/release/team >> release.log &
