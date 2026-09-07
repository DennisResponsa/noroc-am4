#!/bin/sh
set -eu
API_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export AM4_DATA_DIR="$API_ROOT/vendor/am4/assets"
exec "$API_ROOT/bin/noroc-am4-api"
