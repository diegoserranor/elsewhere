#!/usr/bin/env bash
set -euo pipefail

dir="$(cd "$(dirname "$0")/.." && pwd)/data/raw"
mkdir -p "$dir"

base="https://github.com/nvkelso/natural-earth-vector/raw/master/geojson"

curl -fsSL -o "$dir/ne_110m_land.geojson" "$base/ne_110m_land.geojson"

sha256sum "$dir/ne_110m_land.geojson"
