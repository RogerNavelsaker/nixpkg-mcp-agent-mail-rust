#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd -- "$script_dir/.." && pwd)"
manifest_path="$repo_root/nix/package-manifest.json"
tmpdir="$(mktemp -d)"

cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require git
require jq
require rsync

homepage="$(jq -r '.meta.homepage' "$manifest_path")"
default_branch="$(jq -r '.source.defaultBranch // "main"' "$manifest_path")"

if [[ ! "$homepage" =~ ^https://github\.com/([^/]+)/([^/#]+) ]]; then
  echo "failed to parse GitHub owner/repo from homepage: $homepage" >&2
  exit 1
fi

owner="${BASH_REMATCH[1]}"
repo="${BASH_REMATCH[2]}"
source_repo="${1:-https://github.com/$owner/$repo.git}"
source_ref="${2:-$default_branch}"
upstream_dir="$tmpdir/upstream"

echo "syncing $source_repo @ $source_ref"
git clone --depth 1 --branch "$source_ref" "$source_repo" "$upstream_dir" >/dev/null 2>&1
rev="$(git -C "$upstream_dir" rev-parse HEAD)"
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$upstream_dir/Cargo.toml" | head -n 1)"

if [[ -z "$version" ]]; then
  echo "failed to determine version from $upstream_dir/Cargo.toml" >&2
  exit 1
fi

rm -rf "$repo_root/upstream"
mkdir -p "$repo_root/upstream"
rsync -a --delete --exclude '.git' "$upstream_dir/" "$repo_root/upstream/"

tmp_manifest="$manifest_path.tmp"
jq \
  --arg version "$version" \
  --arg rev "$rev" \
  --arg branch "$source_ref" \
  '.source.path = "upstream"
   | .source.channel = "github-head"
   | .source.defaultBranch = $branch
   | .source.version = $version
   | .source.rev = $rev
   | .package.version = $version' \
  "$manifest_path" > "$tmp_manifest"

while IFS=$'\t' read -r sibling _; do
  [[ -n "$sibling" ]] || continue
  sibling_repo="https://github.com/$owner/$sibling.git"
  sibling_dir="$tmpdir/$sibling"
  echo "syncing sibling $sibling_repo @ $source_ref"
  git clone --depth 1 --branch "$source_ref" "$sibling_repo" "$sibling_dir" >/dev/null 2>&1
  sibling_rev="$(git -C "$sibling_dir" rev-parse HEAD)"
  rm -rf "$repo_root/$sibling"
  mkdir -p "$repo_root/$sibling"
  rsync -a --delete --exclude '.git' "$sibling_dir/" "$repo_root/$sibling/"
  jq --arg sibling "$sibling" --arg rev "$sibling_rev" '.source.siblings[$sibling] = $rev' "$tmp_manifest" > "$tmp_manifest.next"
  mv "$tmp_manifest.next" "$tmp_manifest"
done < <(jq -r '.source.siblings | to_entries[]? | [.key, .value] | @tsv' "$manifest_path")

mv "$tmp_manifest" "$manifest_path"

echo "updated:"
echo "  source:   $source_repo"
echo "  ref:      $source_ref"
echo "  rev:      $rev"
echo "  version:  $version"
