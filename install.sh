#!/bin/bash
set -euo pipefail

REPO="raissonsouto/drpizza"
BINARY_NAME="drpizza"

info() { printf '\033[1;34m%s\033[0m\n' "$1"; }
error() { printf '\033[1;31mErro: %s\033[0m\n' "$1" >&2; exit 1; }
success() { printf '\033[1;32m%s\033[0m\n' "$1"; }

# Detect OS
OS="$(uname -s)"
case "$OS" in
    Linux)  OS_NAME="linux" ;;
    Darwin) OS_NAME="macos" ;;
    *)      error "Sistema operacional nao suportado: $OS" ;;
esac

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
    x86_64|amd64)   ARCH_NAME="amd64" ;;
    aarch64|arm64)   ARCH_NAME="arm64" ;;
    *)               error "Arquitetura nao suportada: $ARCH" ;;
esac

# Get latest version
info "Buscando ultima versao..."
VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$VERSION" ]; then
    error "Nao foi possivel encontrar a ultima versao. Verifique sua conexao."
fi

info "Versao encontrada: $VERSION"

# Build asset name
ASSET_NAME="${BINARY_NAME}_${OS_NAME}_${ARCH_NAME}"
DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET_NAME}"

# Choose install directory
if [ -w /usr/local/bin ]; then
    INSTALL_DIR="/usr/local/bin"
elif [ -d "$HOME/.local/bin" ]; then
    INSTALL_DIR="$HOME/.local/bin"
else
    mkdir -p "$HOME/.local/bin"
    INSTALL_DIR="$HOME/.local/bin"
fi

# Download
info "Baixando ${ASSET_NAME}..."
TMP_FILE=$(mktemp)
HTTP_CODE=$(curl -fsSL -w "%{http_code}" -o "$TMP_FILE" "$DOWNLOAD_URL" 2>/dev/null || true)

if [ "$HTTP_CODE" != "200" ] || [ ! -s "$TMP_FILE" ]; then
    rm -f "$TMP_FILE"
    error "Falha ao baixar o binario. URL: ${DOWNLOAD_URL}\nVerifique se existe um release para ${OS_NAME}/${ARCH_NAME}."
fi

# Install
chmod +x "$TMP_FILE"
mv "$TMP_FILE" "${INSTALL_DIR}/${BINARY_NAME}"

success "drpizza ${VERSION} instalado em ${INSTALL_DIR}/${BINARY_NAME}"

# Check if install dir is in PATH
case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        echo ""
        echo "Adicione ${INSTALL_DIR} ao seu PATH:"
        echo "  export PATH=\"${INSTALL_DIR}:\$PATH\""
        echo ""
        ;;
esac

info "Execute 'drpizza' para comecar!"
