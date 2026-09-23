#!/bin/sh
# Installs the latest Trail release on Linux and macOS.
#
#   curl -fsSL https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.sh | sh
#
# TRAIL_INSTALL_DIR overrides the install directory (default: ~/.local/bin).
# TRAIL_VERSION installs a specific release tag, e.g. v0.6.0 (default: latest).

set -eu

REPO="KayraBulbul/Trail"

err() {
    echo "error: $*" >&2
    exit 1
}

need() {
    command -v "$1" >/dev/null 2>&1 || err "$1 is required to install Trail"
}

# Everything runs inside main so a partially downloaded script never executes.
main() {
    need curl
    need tar
    need uname

    case "$(uname -s)" in
        Linux) os="linux" ;;
        Darwin) os="macos" ;;
        *) err "unsupported OS: $(uname -s). On Windows, use install.ps1" ;;
    esac

    case "$(uname -m)" in
        x86_64 | amd64) arch="x86_64" ;;
        arm64 | aarch64) arch="aarch64" ;;
        *) err "unsupported architecture: $(uname -m)" ;;
    esac

    if [ "$os" = "linux" ] && [ "$arch" = "aarch64" ]; then
        err "there is no Linux ARM build yet"
    fi

    asset="trail-${os}-${arch}.tar.gz"
    version="${TRAIL_VERSION:-latest}"
    if [ "$version" = "latest" ]; then
        base_url="https://github.com/${REPO}/releases/latest/download"
    else
        base_url="https://github.com/${REPO}/releases/download/${version}"
    fi
    install_dir="${TRAIL_INSTALL_DIR:-$HOME/.local/bin}"

    tmp_dir="$(mktemp -d)"
    trap 'rm -rf "$tmp_dir"' EXIT INT TERM

    echo "Downloading ${asset} (${version})..."
    curl -fsSL "${base_url}/${asset}" -o "${tmp_dir}/${asset}" ||
        err "couldn't download ${asset}"
    curl -fsSL "${base_url}/SHA256SUMS.txt" -o "${tmp_dir}/SHA256SUMS.txt" ||
        err "couldn't download SHA256SUMS.txt"

    expected="$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "${tmp_dir}/SHA256SUMS.txt")"
    [ -n "$expected" ] || err "SHA256SUMS.txt has no entry for ${asset}"

    if command -v sha256sum >/dev/null 2>&1; then
        actual="$(sha256sum "${tmp_dir}/${asset}" | awk '{ print $1 }')"
    elif command -v shasum >/dev/null 2>&1; then
        actual="$(shasum -a 256 "${tmp_dir}/${asset}" | awk '{ print $1 }')"
    else
        err "sha256sum or shasum is required to verify the download"
    fi
    [ "$actual" = "$expected" ] || err "checksum mismatch for ${asset}, aborting"

    tar -xzf "${tmp_dir}/${asset}" -C "$tmp_dir"
    [ -f "${tmp_dir}/trail" ] || err "release archive doesn't contain trail"

    mkdir -p "$install_dir"
    # Install to a temp name first so the old binary is only replaced once the copy succeeds.
    cp "${tmp_dir}/trail" "${install_dir}/.trail.new"
    chmod 755 "${install_dir}/.trail.new"
    mv "${install_dir}/.trail.new" "${install_dir}/trail"

    echo "Installed Trail to ${install_dir}/trail"

    case ":${PATH}:" in
        *":${install_dir}:"*) echo "Run 'trail' to get started." ;;
        *)
            echo
            echo "${install_dir} is not on your PATH. Add this to your shell config:"
            echo "    export PATH=\"${install_dir}:\$PATH\""
            ;;
    esac
}

main "$@"
