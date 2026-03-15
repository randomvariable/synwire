#!/usr/bin/env bash
# Pull the default model for E2E testing.
set -euo pipefail

MODEL="${OLLAMA_MODEL:-llama3.2:1b}"
OLLAMA_HOST="${OLLAMA_HOST:-http://localhost:11434}"

echo "Pulling model ${MODEL} from ${OLLAMA_HOST}..."
curl -sf "${OLLAMA_HOST}/api/pull" -d "{\"name\": \"${MODEL}\"}"
echo "Model ${MODEL} pulled successfully."
