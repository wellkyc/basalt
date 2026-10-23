#!/bin/sh
set -eu

repo="wellkyc/basalt"
dir="${BASALT_INSTALL_DIR:-$HOME/.local/bin}"

case "$(uname -s)" in
    Linux) os=linux ;;
    Darwin) os=macos ;;
    *)
        echo "basalt: $(uname -s) is not supported by this script, on Windows use install.ps1" >&2
        exit 1
        ;;
esac

case "$(uname -m)" in
    x86_64 | amd64) arch=x86_64 ;;
    arm64 | aarch64) arch=arm64 ;;
    *)
        echo "basalt: no prebuilt binary for $(uname -m), try: cargo install --git https://github.com/$repo" >&2
        exit 1
        ;;
esac

url="https://github.com/$repo/releases/latest/download/basalt-$os-$arch.tar.gz"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading $url"
curl -fsSL "$url" -o "$tmp/basalt.tar.gz"
tar -xzf "$tmp/basalt.tar.gz" -C "$tmp"
mkdir -p "$dir"
mv "$tmp/basalt" "$dir/basalt"
chmod +x "$dir/basalt"
echo "Installed basalt to $dir/basalt"

case ":$PATH:" in
    *":$dir:"*) ;;
    *)
        echo
        echo "$dir is not in your PATH yet. Add it with:"
        echo "  echo 'export PATH=\"$dir:\$PATH\"' >> ~/.profile"
        ;;
esac
