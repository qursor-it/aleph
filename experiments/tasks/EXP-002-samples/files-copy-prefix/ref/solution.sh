#!/usr/bin/env bash
while IFS= read -r line; do
  case "$line" in
    "$1"*) printf '%s\n' "$line" ;;
  esac
done < in.txt > out.txt
