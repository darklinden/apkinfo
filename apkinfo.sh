#!/usr/bin/env bash

BASEDIR=$(dirname "$0")
SCRIPT_DIR="$(realpath "${BASEDIR}")"

echo "Script directory: $SCRIPT_DIR"

GLOBAL_PYTHON=$(which python3)
if [ -z "$GLOBAL_PYTHON" ]; then
    GLOBAL_PYTHON=$(which python)
    if [ -z "$GLOBAL_PYTHON" ]; then
        echo "Python is not installed"
        exit 1
    else
        PYTHON_VERSION=$($GLOBAL_PYTHON --version)
        if ! [[ $PYTHON_VERSION == *"3."* ]]; then
            echo "Python 3 is not installed"
            exit 1
        fi
    fi
else
    PYTHON_VERSION=$($GLOBAL_PYTHON --version)
    if ! [[ $PYTHON_VERSION == *"3."* ]]; then
        echo "Python 3 is not installed"
        exit 1
    fi
fi

echo "Using global python: $GLOBAL_PYTHON $PYTHON_VERSION"

VENV_DIR=$SCRIPT_DIR"/venv"
ACTIVATE=$VENV_DIR"/bin/activate"

if [ ! -d $VENV_DIR ]; then
    # Take action if $DIR exists. #
    mkdir $VENV_DIR
    $GLOBAL_PYTHON -m venv $VENV_DIR
else
    echo "Virtual environment folder already exists"
fi

if grep -Faq "$VENV_DIR" $ACTIVATE; then
    # code if found
    echo "Virtual environment is ready to use"
else
    # code if not found
    echo "Virtual environment is incorrect, recreating..."

    rm -rf $VENV_DIR

    # Take action if $DIR exists. #
    mkdir $VENV_DIR
    $GLOBAL_PYTHON -m venv $VENV_DIR
fi

source $ACTIVATE
pip install -r requirements.txt

python $SCRIPT_DIR"/apkinfo.py" $@
