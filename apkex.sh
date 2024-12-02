#!/usr/bin/env bash

SCRIPT_DIR=$(dirname "$0")
SCRIPT_DIR=$(realpath "$SCRIPT_DIR")

# if cygpath is available, convert the path to windows format
if command -v cygpath >/dev/null; then
    EXEC_PATH=$(cygpath -wa "$SCRIPT_DIR/apkex")
    echo "EXEC_PATH: $EXEC_PATH"
    rust-script "$EXEC_PATH" $@
else
    # otherwise, just use the path as is
    rust-script "$SCRIPT_DIR/apkex" $@
fi
