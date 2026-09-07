#!/usr/bin/env bash
# Run a dev instance on this VM against a COPY of the production database, so a
# human can drive the app with real trips and real places.
#
# The production database at PROD_HOST is copy-from only (see CLAUDE.local.md).
# This script never writes to it. The copy lives in data-dev/, which .gitignore
# already excludes via /data-*/ - it holds real addresses and must never reach
# this public repo.
set -euo pipefail

PROD_HOST="${PROD_HOST:-root@192.168.0.112}"
PROD_DB="${PROD_DB:-/root/kniha-jazd/data/kniha-jazd.db}"
PORT="${PORT:-3460}"
NAME="kniha-jazd-dev"
IMAGE="kniha-jazd-web:dev"

cd "$(dirname "$0")/.."
REPO="$PWD"

echo "==> Building the image from the working tree"
docker build -f Dockerfile.web -t "$IMAGE" .

echo "==> Copying the production database (read-only source)"
mkdir -p "$REPO/data-dev"
scp -o BatchMode=yes "$PROD_HOST:$PROD_DB" "$REPO/data-dev/kniha-jazd.db"
chmod u+w "$REPO/data-dev/kniha-jazd.db"

echo "==> Starting $NAME on port $PORT"
docker rm -f "$NAME" >/dev/null 2>&1 || true
docker run -d --name "$NAME" \
  -p "$PORT:3456" \
  -v "$REPO/data-dev:/data" \
  -e KNIHA_JAZD_DATA_DIR=/data \
  -e DATABASE_PATH=/data/kniha-jazd.db \
  -e PORT=3456 \
  "$IMAGE" >/dev/null

echo "==> Waiting for health"
for _ in $(seq 1 60); do
  if curl -sf "http://localhost:$PORT/health" >/dev/null 2>&1; then
    echo "    healthy"
    break
  fi
  sleep 1
done

echo "==> Data check"
curl -s -X POST "http://localhost:$PORT/api/rpc" \
  -H 'Content-Type: application/json' -H 'X-KJ-Client: 1' \
  -d '{"command":"get_vehicles","args":{}}' |
  python3 -c 'import sys,json; print(f"    vehicles: {len(json.load(sys.stdin))}")'

echo
echo "Open: http://ubuntu.lacny.me:$PORT/"
echo "Stop: docker rm -f $NAME"
