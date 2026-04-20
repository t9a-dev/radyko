#!/bin/sh
set -eu

mkdir -p ./recorded

if [ ! -f .env ]; then
  cp .env.example .env
  {
    echo ""
    echo "PUID=$(id -u)"
    echo "PGID=$(id -g)"
  } >> .env
fi

if [ ! -f compose.yml ]; then
  cp example.compose.yml compose.yml
fi

echo "Initialized .env, compose.yml and ./recorded"