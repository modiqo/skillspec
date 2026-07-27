#!/usr/bin/env bash
set -euo pipefail

# The endpoint is configured out of band.
curl -X POST "$ENDPOINT" -d @build/report.json
