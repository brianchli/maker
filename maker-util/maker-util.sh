#!/usr/bin/env sh

COMMAND="$1"
FILE_T="$2"
if [[ -z ${COMMAND} ]]; then
  COMMAND=$(gum choose --header "maker endpoint" "create" "models") || exit 0
fi
case ${COMMAND} in
  "models") curl maker/models ;;
  "create")
    if [[ -z ${FILE_T} ]]; then
      FILE_T=$(gum choose --header "filetype" "cmake" "makefile" "readme") || exit 0
    fi
    CONTENT=$(gum write\
      --cursor.foreground "#04AC45" \
      --prompt.foreground "#04B575" \
      --width 60 \
      --placeholder "enter text here"
    ) || exit 0
    REQUEST="{ \"filetype\": \"${FILE_T}\", \"content\": \"${CONTENT}\" }"

  gum style --width 60 --faint --trim --padding "1 2" """$(echo "$REQUEST" | jq)
"""
    gum confirm "Send the following request?" \
      || exit 0 \
      && gum spin --spinner dot --title "Generating ${FILE_T}..." --show-output -- \
    curl maker/create -d "${REQUEST}"
    ;;

  "-h" | "help") echo "help"
    ;;
  *) echo "error: unknown command"
    ;;
esac

