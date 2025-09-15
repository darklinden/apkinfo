#!/usr/bin/env bash

cargo build --release

mv target/release/adblog ./adblog
mv target/release/apkex ./apkex
mv target/release/apkinfo ./apkinfo

cargo clean

echo "Build finished."
