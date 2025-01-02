#!/usr/bin/env bash
set -euo pipefail

if [[ $# -eq 0 ]] && ! tty -s; then
	cd "$(dirname "$(realpath "$0")")"
	COLUMNS=80 env -C ../teNOR cargo r -q -- -h >src/teNOR.help </dev/null
	jq '.[1]'
elif [[ "${1-}" = "supports" ]]; then
	exit 0 # All renderers are supported.
else
	echo 'This script is only meant to be executed as a mdBook preprocessor.' >&2
	exit 1
fi
