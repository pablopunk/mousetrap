#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")" && pwd)"
keys=(1 2 3 4 5 6 7 8 9 0 q w e r t y u i o p a s d f g h j k l semicolon z x c v b n m comma period slash)

hyprctl keyword unbind ", escape" >/dev/null 2>&1 || true
for key in "${keys[@]}"; do
  hyprctl keyword unbind ", ${key}" >/dev/null 2>&1 || true
done

hyprctl keyword submap mousetrap
hyprctl keyword binde ", escape, exec, ${ROOT}/cancel.sh"
for key in "${keys[@]}"; do
  case "$key" in
    semicolon) arg=';' ;;
    comma) arg=',' ;;
    period) arg='.' ;;
    slash) arg='/' ;;
    *) arg="$key" ;;
  esac
  hyprctl keyword binde ", ${key}, exec, ${ROOT}/select.sh ${arg}"
done
hyprctl dispatch submap mousetrap
