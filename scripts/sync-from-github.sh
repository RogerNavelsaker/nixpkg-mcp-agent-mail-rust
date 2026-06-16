#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
manifest_path="$repo_root/nix/package-manifest.json"

require() {
  command -v "$1" >/dev/null 2>&1 || { echo "missing required command: $1" >&2; exit 1; }
}
require git
require jq
require nix
require curl

nix_hash_of() {
  local owner="$1" repo="$2" rev="$3"
  local url="https://github.com/$owner/$repo/archive/$rev.tar.gz"
  local raw
  raw="$(nix-prefetch-url --unpack "$url" 2>/dev/null | tail -n 1)"
  nix hash to-sri --type sha256 "$raw"
}

latest_head() {
  local owner="$1" repo="$2" branch="$3"
  git ls-remote "https://github.com/$owner/$repo.git" "refs/heads/$branch" \
    | cut -f1 | head -n1
}

latest_tag() {
  local owner="$1" repo="$2"
  git ls-remote --tags --refs "https://github.com/$owner/$repo.git" 'v*' \
    | awk -F/ '{print $3}' | sort -V | tail -n 1
}

detect_version() {
  local owner="$1" repo="$2" rev="$3" fallback="$4"
  local base="https://raw.githubusercontent.com/$owner/$repo/$rev"
  local v
  if v="$(curl -sfL "$base/Cargo.toml" | sed -n 's/^version = "\([^"]*\)"/\1/p' | head -n1)" && [[ -n "$v" ]]; then
    echo "$v"; return
  fi
  if v="$(curl -sfL "$base/package.json" | python3 -c 'import sys,json; d=json.load(sys.stdin); print(d.get("version",""))' 2>/dev/null)" && [[ -n "$v" ]]; then
    echo "$v"; return
  fi
  if v="$(curl -sfL "$base/pyproject.toml" | sed -n 's/^version = "\([^"]*\)"/\1/p' | head -n1)" && [[ -n "$v" ]]; then
    echo "$v"; return
  fi
  echo "$fallback"
}

owner="$(jq -r '.source.owner' "$manifest_path")"
repo="$(jq -r '.source.repo' "$manifest_path")"
channel="$(jq -r '.source.channel // "github-head"' "$manifest_path")"
default_branch="$(jq -r '.source.defaultBranch // "main"' "$manifest_path")"
current_version="$(jq -r '.package.version' "$manifest_path")"

if [[ "$channel" == "github-release" ]]; then
  tag="${1:-$(latest_tag "$owner" "$repo")}"
  if [[ -z "$tag" ]]; then
    echo "failed to find latest release tag for $owner/$repo" >&2
    exit 1
  fi
  rev="$(git ls-remote "https://github.com/$owner/$repo.git" "refs/tags/$tag" | cut -f1 | head -n1)"
  version="${tag#v}"
  ref="$tag"
else
  ref="${1:-$default_branch}"
  rev="$(latest_head "$owner" "$repo" "$ref")"
  version="$(detect_version "$owner" "$repo" "$rev" "$current_version")"
fi

if [[ -z "$rev" ]]; then
  echo "failed to resolve ref: $ref for $owner/$repo" >&2
  exit 1
fi

echo "syncing $owner/$repo @ $ref"
echo "  rev: $rev"
hash="$(nix_hash_of "$owner" "$repo" "$rev")"
echo "  hash: $hash"
echo "  version: $version"

jq \
  --arg rev "$rev" \
  --arg hash "$hash" \
  --arg version "$version" \
  '.source.rev = $rev | .source.hash = $hash | .source.version = $version | .package.version = $version' \
  "$manifest_path" > "$manifest_path.tmp"
mv "$manifest_path.tmp" "$manifest_path"

if [[ "$channel" == "github-release" ]]; then
  jq --arg tag "$ref" '.source.tag = $tag' "$manifest_path" > "$manifest_path.tmp"
  mv "$manifest_path.tmp" "$manifest_path"
fi

while IFS= read -r sibling_key; do
  [[ -n "$sibling_key" ]] || continue
  s_owner="$(jq -r --arg k "$sibling_key" '.source.siblings[$k].owner' "$manifest_path")"
  s_repo="$(jq -r --arg k "$sibling_key" '.source.siblings[$k].repo' "$manifest_path")"
  s_branch="$(jq -r --arg k "$sibling_key" '.source.siblings[$k].defaultBranch // "main"' "$manifest_path")"
  echo "syncing sibling $sibling_key ($s_owner/$s_repo @ $s_branch)"
  s_rev="$(latest_head "$s_owner" "$s_repo" "$s_branch")"
  s_hash="$(nix_hash_of "$s_owner" "$s_repo" "$s_rev")"
  echo "  rev: $s_rev  hash: $s_hash"
  jq \
    --arg k "$sibling_key" \
    --arg rev "$s_rev" \
    --arg hash "$s_hash" \
    '.source.siblings[$k].rev = $rev | .source.siblings[$k].hash = $hash' \
    "$manifest_path" > "$manifest_path.tmp"
  mv "$manifest_path.tmp" "$manifest_path"
done < <(jq -r '.source.siblings // {} | keys[]' "$manifest_path")

echo "done."
