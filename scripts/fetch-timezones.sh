#!/usr/bin/env bash
set -euo pipefail

dir="$(cd "$(dirname "$0")/.." && pwd)/data/raw"
mkdir -p "$dir"

# Pinned, so a regenerated data/zones.tsv comes from the release recorded in
# data/manifest.toml. Bump with each tzdata release.
release="2026e"
base="https://github.com/evansiroky/timezone-boundary-builder/releases/download/$release"

for name in timezones-with-oceans-now timezones-now; do
	curl -fsSL -o "$dir/$name.geojson.zip" "$base/$name.geojson.zip"
	unzip -o -q "$dir/$name.geojson.zip" -d "$dir"
done

sha256sum "$dir/timezones-with-oceans-now.geojson.zip" "$dir/timezones-now.geojson.zip"
