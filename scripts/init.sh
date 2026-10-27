#!/usr/bin/env bash
# 下载/更新 mihomo 内核到 clash-meta/mihomo，按当前系统和架构获取最新 release
set -euo pipefail

REPO="MetaCubeX/mihomo"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/clash-meta/mihomo"
FORCE="${1:-}"

case "$(uname -s)" in
    Darwin) os=darwin ;;
    Linux) os=linux ;;
    *)
        echo "unsupported OS: $(uname -s)" >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) arch=amd64 ;;
    arm64 | aarch64) arch=arm64 ;;
    *)
        echo "unsupported arch: $(uname -m)" >&2
        exit 1
        ;;
esac

tag="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")"
tag="${tag##*/}"
if [[ "$tag" != v* ]]; then
    echo "failed to resolve latest mihomo version" >&2
    exit 1
fi

if [[ "$FORCE" != "--force" && -x "$BIN" ]]; then
    current="$("$BIN" -v 2>/dev/null || true)"
    if [[ "$current" == *"$tag"* ]]; then
        echo "mihomo $tag already installed"
        exit 0
    fi
fi

asset="mihomo-$os-$arch-$tag.gz"
echo "downloading mihomo $tag ($os-$arch)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
curl -fsSL --retry 3 -o "$tmp/mihomo.gz" \
    "https://github.com/$REPO/releases/download/$tag/$asset"
gzip -dc "$tmp/mihomo.gz" >"$tmp/mihomo"
chmod +x "$tmp/mihomo"
mkdir -p "$(dirname "$BIN")"
mv "$tmp/mihomo" "$BIN"
"$BIN" -v | head -1
