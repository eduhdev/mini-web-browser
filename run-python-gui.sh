#!/bin/zsh

cd "$(dirname "$0")/python" || exit 1
PYTHON=/opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/bin/python3.12

if [[ "$1" == "--profile" ]]; then
  shift
  exec "$PYTHON" -m cProfile -s cumulative -m browser.graphics "$@"
fi

exec "$PYTHON" -m browser.graphics "$@"
