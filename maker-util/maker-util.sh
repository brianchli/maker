#!/usr/bin/env bash

set -euo pipefail
IFS=$'\n\t'

trap 'exit 0' INT TERM

for dep in curl gum jq; do
  if ! command -v "$dep" >/dev/null 2>&1; then
    printf '\033[31merror\033[0m: %s is required but not installed\n' "$dep" >&2
    exit 1
  fi
done

MAKER_ENDPOINT="${MAKER_ENDPOINT:-maker}"
CURL_OPTS=(-s --fail -H "Accept: application/json")

_maker_usage() {
  cat <<EOF
maker-util: ergonomic CLI for invoking the Maker Endpoint

Usage:
  maker-util [command] [args]

Commands:
  create [filetype]     Generate a file
  models                List available models
  --help | -h              Show this help

Environment:
  MAKER_ENDPOINT       Base URL for the Maker API (default: maker)
EOF
}

COMMAND="${1:-}"
FILE_T="${2:-}"

if [[ -z ${COMMAND} ]]; then
  COMMAND=$(gum choose --header "maker endpoint" "create" "models") || exit 0
fi

case ${COMMAND} in
  "models")
    curl "${CURL_OPTS[@]}" "${MAKER_ENDPOINT}/models" | jq -r '
  ["MODEL","REMOTE","HOST","SIZE","UPDATED","DIGEST"],
  (.models[] | [
    .model,
    .remote_model,
    .remote_host[8:],
    (.size | tostring),
    (.modified_at[0:16]),
    (.digest[0:16] + "...")
  ]) | @tsv
' | column -t -s $'\t' | gum table || exit 0
    ;;

  "create")
    if [[ -z ${FILE_T} ]]; then
      FILE_T=$(curl -fs ${MAKER_ENDPOINT}/specs 2>/dev/null | jq -r '.[]' | gum choose --header "filetype") || exit 0
    fi

    CONTENT=$(gum write \
      --cursor.foreground "#04AC45" \
      --prompt.foreground "#04B575" \
      --width 60 \
      --placeholder "enter text here"
    ) || exit 0

    if [[ -z ${CONTENT} ]]; then
      printf '%s: content cannot be empty\n' "$(gum style --foreground '#FF0000' 'error')" >&2
      exit 1
    fi

    REQUEST=$(jq -n --arg ft "$FILE_T" --arg c "$CONTENT" '{filetype: $ft, content: $c}')

    gum style --width 60 --faint --trim --padding "1 2" "$REQUEST"

    if gum confirm "Send the following request?"; then
      gum spin --spinner dot --title "generating ${FILE_T}..." --show-output -- \
        curl "${CURL_OPTS[@]}" -H "Content-Type: application/json" -d "$REQUEST" "${MAKER_ENDPOINT}/create"
    fi
    ;;

  "-h" | "--help")
    _maker_usage
    ;;

  *)
    printf '%s: unknown command '\''%s'\''\n' "$(gum style --foreground '#FF0000' 'error')" "${COMMAND}" >&2
    _maker_usage >&2
    exit 1
    ;;
esac
