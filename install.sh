#!/bin/sh
set -e

REPO="berlin-566/Wee"
INSTALL_DIR="${HOME}/.local/bin"
BINARY_NAME="wee"

mkdir -p "${INSTALL_DIR}"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

if [ "${OS}" != "linux" ] || [ "${ARCH}" != "x86_64" ]; then
    echo "Currently, prebuilt release binaries are provided for Linux x86_64."
    echo "To compile from source, run: cargo install --git https://github.com/${REPO}.git"
    exit 1
fi

echo "Fetching latest release of wee..."
DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/wee-linux-x86_64.tar.gz"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "${TMP_DIR}"' EXIT

curl -fsSL "${DOWNLOAD_URL}" -o "${TMP_DIR}/wee.tar.gz"
tar -xzf "${TMP_DIR}/wee.tar.gz" -C "${TMP_DIR}"

install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

echo "Successfully installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME}"

case ":${PATH}:" in
    *:"${INSTALL_DIR}":*) ;;
    *) echo "Note: Add ${INSTALL_DIR} to your PATH to run '${BINARY_NAME}' from anywhere." ;;
esac
