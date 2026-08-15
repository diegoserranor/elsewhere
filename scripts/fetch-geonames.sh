#!/usr/bin/env bash
set -euo pipefail

dir="$(cd "$(dirname "$0")/.." && pwd)/data/raw"
mkdir -p "$dir"

base="https://download.geonames.org/export/dump"

curl -fsSL -o "$dir/cities5000.zip" "$base/cities5000.zip"
unzip -oq "$dir/cities5000.zip" cities5000.txt -d "$dir"
rm -f "$dir/cities5000.zip"

curl -fsSL -o "$dir/countryInfo.txt" "$base/countryInfo.txt"
curl -fsSL -o "$dir/admin1CodesASCII.txt" "$base/admin1CodesASCII.txt"

sha256sum "$dir/cities5000.txt" "$dir/countryInfo.txt" "$dir/admin1CodesASCII.txt"
