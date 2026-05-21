#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-nihui/realcugan-ncnn-vulkan}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET_DIR="$SCRIPT_DIR/src-tauri/binaries/models-se"

IFS="|" read -r ASSET_URL ASSET_NAME <<< "$(python3 - <<'PY'
import json
import os
import sys
import urllib.request

repo = os.environ.get("REPO", "nihui/realcugan-ncnn-vulkan")
url = f"https://api.github.com/repos/{repo}/releases/latest"
with urllib.request.urlopen(url) as resp:
    data = json.load(resp)
assets = data.get("assets", [])

def score(name: str) -> int:
    if name.endswith(".zip"):
        return 0
    if name.endswith(".7z"):
        return 1
    return 2

cands = [a for a in assets if "models-se" in a.get("name", "")]
cands.sort(key=lambda a: score(a.get("name", "")))
if not cands:
    print(f"No models-se asset found for {repo}.", file=sys.stderr)
    sys.exit(1)

print(f"{cands[0]['browser_download_url']}|{cands[0]['name']}")
PY
)"

if [[ -z "$ASSET_URL" || -z "$ASSET_NAME" ]]; then
  echo "Failed to resolve models-se download URL." >&2
  exit 1
fi

mkdir -p "$TARGET_DIR"
ARCHIVE="$TARGET_DIR/$ASSET_NAME"

echo "Downloading $ASSET_NAME..."
curl -L -o "$ARCHIVE" "$ASSET_URL"

if [[ "$ASSET_NAME" == *.zip ]]; then
  if ! command -v unzip >/dev/null 2>&1; then
    echo "unzip not found; install unzip to extract .zip." >&2
    exit 1
  fi
  unzip -o "$ARCHIVE" -d "$TARGET_DIR"
elif [[ "$ASSET_NAME" == *.7z ]]; then
  if ! command -v 7z >/dev/null 2>&1; then
    echo "7z not found; install p7zip or 7-Zip to extract .7z." >&2
    exit 1
  fi
  7z x "$ARCHIVE" -o"$TARGET_DIR" -y >/dev/null
else
  echo "Unsupported archive type: $ASSET_NAME" >&2
  exit 1
fi

rm -f "$ARCHIVE"

if [[ -d "$TARGET_DIR/models-se" ]]; then
  shopt -s dotglob
  mv "$TARGET_DIR/models-se"/* "$TARGET_DIR"/
  shopt -u dotglob
  rmdir "$TARGET_DIR/models-se" || true
fi

echo "Done: $TARGET_DIR"
