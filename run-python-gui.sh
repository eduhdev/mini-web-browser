#!/bin/zsh

cd "$(dirname "$0")/python" || exit 1
/opt/homebrew/Cellar/python@3.12/3.12.13/Frameworks/Python.framework/Versions/3.12/bin/python3.12 -m browser.graphics "$@"
