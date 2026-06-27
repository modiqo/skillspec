#!/usr/bin/env sh
set -eu

REPO="${SKILLSPEC_REPO:-modiqo/skillspec}"
VERSION="${SKILLSPEC_VERSION:-latest}"
INSTALL_DIR="${SKILLSPEC_INSTALL_DIR:-$HOME/.local/bin}"

case "$VERSION" in
  latest)
    RELEASE_URL="https://github.com/${REPO}/releases/latest/download"
    ;;
  v*)
    RELEASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"
    ;;
  *)
    RELEASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
    ;;
esac

OS="$(uname -s)"
ARCH="$(uname -m)"

case "${OS}:${ARCH}" in
  Darwin:*)
    ASSET="skillspec-macos.tar.gz"
    ;;
  Linux:x86_64|Linux:amd64)
    ASSET="skillspec-linux-x86_64.tar.gz"
    ;;
  *)
    echo "Unsupported platform for the prebuilt SkillSpec binary: ${OS} ${ARCH}" >&2
    echo "Try: cargo install skillspec" >&2
    exit 1
    ;;
esac

TMPDIR="$(mktemp -d)"
cleanup() {
  rm -rf "$TMPDIR"
}
trap cleanup EXIT INT TERM

ARCHIVE="${TMPDIR}/${ASSET}"
CHECKSUM="${ARCHIVE}.sha256"

echo "Downloading ${ASSET} from ${RELEASE_URL}"
curl -fsSL "${RELEASE_URL}/${ASSET}" -o "$ARCHIVE"
curl -fsSL "${RELEASE_URL}/${ASSET}.sha256" -o "$CHECKSUM"

echo "Verifying checksum"
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$TMPDIR" && sha256sum -c "${ASSET}.sha256")
elif command -v shasum >/dev/null 2>&1; then
  (cd "$TMPDIR" && shasum -a 256 -c "${ASSET}.sha256")
else
  echo "Neither sha256sum nor shasum is available; cannot verify download." >&2
  exit 1
fi

tar -xzf "$ARCHIVE" -C "$TMPDIR"
mkdir -p "$INSTALL_DIR"

if command -v install >/dev/null 2>&1; then
  install -m 0755 "${TMPDIR}/skillspec" "${INSTALL_DIR}/skillspec"
else
  cp "${TMPDIR}/skillspec" "${INSTALL_DIR}/skillspec"
  chmod 0755 "${INSTALL_DIR}/skillspec"
fi

echo "Installed skillspec to ${INSTALL_DIR}/skillspec"
"${INSTALL_DIR}/skillspec" --version

case ":$PATH:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    echo "Note: ${INSTALL_DIR} is not on PATH." >&2
    echo "Add it to PATH or run ${INSTALL_DIR}/skillspec directly." >&2
    ;;
esac
