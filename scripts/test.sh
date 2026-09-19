#!/bin/sh
set -eu
cd "$(dirname "$0")/.."

case "${1:-all}" in
  all)
    docker compose run --build --rm tools cargo test --locked
    docker compose run --build --rm browser-tests
    ;;
  unit)
    docker compose run --build --rm tools cargo test --locked
    ;;
  e2e)
    docker compose run --build --rm browser-tests npm run test:e2e
    ;;
  visual)
    docker compose run --build --rm browser-tests npm run test:visual
    ;;
  update)
    docker compose run --build --rm browser-tests npm run test:visual:update
    ;;
  report)
    docker compose up --build -d report
    echo "Playwright report: http://localhost:${TODO_REPORT_PORT:-9324}"
    ;;
  *)
    echo 'Usage: ./scripts/test.sh [all|unit|e2e|visual|update|report]' >&2
    exit 2
    ;;
esac
