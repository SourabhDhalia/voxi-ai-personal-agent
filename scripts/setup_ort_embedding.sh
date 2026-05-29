#!/usr/bin/env bash
# setup_ort_embedding.sh — Downloads ONNX Runtime + MiniLM model files
# into the Voxi data directory for on-device embedding support.
#
# Usage:
#   ./scripts/setup_ort_embedding.sh              # auto-detect platform
#   ./scripts/setup_ort_embedding.sh --data-dir /opt/usr/share/voxi
#
# Supports:
#   - Ubuntu x86_64  (host test environment)
#   - Voxi armv7l   (production target)

set -euo pipefail

ORT_VERSION="${ORT_VERSION:-1.20.1}"
MINILM_REPO="https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main"

# ── Detect platform ──────────────────────────────────────────────────
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)  ORT_ARCH="x64"   ;;
  armv7l)  ORT_ARCH="arm"   ;;
  aarch64) ORT_ARCH="aarch64" ;;
  *)
    echo "ERROR: Unsupported architecture: $ARCH"
    exit 1
    ;;
esac
echo "[INFO] Detected architecture: $ARCH → ORT package: linux-$ORT_ARCH"

# ── Determine data directory ─────────────────────────────────────────
DATA_DIR="${VOXI_DATA_DIR:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --data-dir) DATA_DIR="$2"; shift 2 ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
done

if [[ -z "$DATA_DIR" ]]; then
  if [[ -f /etc/voxi-release ]] || [[ -d /opt/usr/share/voxi ]]; then
    DATA_DIR="/opt/usr/share/voxi"
  else
    DATA_DIR="${HOME}/.voxi"
  fi
fi

LIB_DIR="${DATA_DIR}/lib"
MODEL_DIR="${DATA_DIR}/models"

echo "[INFO] Data directory : $DATA_DIR"
echo "[INFO] Lib directory  : $LIB_DIR"
echo "[INFO] Model directory: $MODEL_DIR"

mkdir -p "$LIB_DIR" "$MODEL_DIR"

# ── Download ONNX Runtime ────────────────────────────────────────────
ORT_TARBALL="onnxruntime-linux-${ORT_ARCH}-${ORT_VERSION}.tgz"
ORT_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ORT_VERSION}/${ORT_TARBALL}"

if [[ -f "${LIB_DIR}/libonnxruntime.so" ]]; then
  echo "[SKIP] libonnxruntime.so already exists in ${LIB_DIR}"
else
  TMPDIR=$(mktemp -d)
  trap 'rm -rf "$TMPDIR"' EXIT

  echo "[DL]   Downloading ORT ${ORT_VERSION} for ${ORT_ARCH}..."
  wget -q --show-progress -O "${TMPDIR}/${ORT_TARBALL}" "$ORT_URL"

  echo "[EXT]  Extracting..."
  tar xzf "${TMPDIR}/${ORT_TARBALL}" -C "$TMPDIR"

  ORT_EXTRACT_DIR="${TMPDIR}/onnxruntime-linux-${ORT_ARCH}-${ORT_VERSION}"
  cp -v "${ORT_EXTRACT_DIR}/lib/libonnxruntime.so"* "${LIB_DIR}/"

  echo "[OK]   ONNX Runtime installed to ${LIB_DIR}"
fi

# ── Download MiniLM model ────────────────────────────────────────────
if [[ -f "${MODEL_DIR}/model.onnx" ]] && [[ -f "${MODEL_DIR}/vocab.txt" ]]; then
  echo "[SKIP] model.onnx and vocab.txt already exist in ${MODEL_DIR}"
else
  echo "[DL]   Downloading all-MiniLM-L6-v2 ONNX model..."
  wget -q --show-progress -O "${MODEL_DIR}/model.onnx" "${MINILM_REPO}/onnx/model.onnx"
  wget -q --show-progress -O "${MODEL_DIR}/vocab.txt"  "${MINILM_REPO}/vocab.txt"
  echo "[OK]   MiniLM model installed to ${MODEL_DIR}"
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo "  Setup complete! Restart the Voxi daemon."
echo "  Expected log on startup:"
echo "    ONNX Runtime found at: ${LIB_DIR}/libonnxruntime.so"
echo "    OnDeviceEmbedding initialized (dim=384)"
echo "═══════════════════════════════════════════════════"
