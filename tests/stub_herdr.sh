#!/usr/bin/env bash
set -u

log="${CALL_LOG:?}"

if [ -z "${STUB_FOCUS+x}" ]; then
  STUB_FOCUS='{"result":{"focus":{"changed":true}}}'
fi

note() { printf '%s\n' "$*" >> "$log"; }
out() { printf '%s' "$1"; }

case "${1:-} ${2:-}" in
"pane process-info")
  out "${STUB_PROCESS_INFO:?}"
  ;;
"pane send-keys")
  note "send-keys $3 $4"
  out '{}'
  ;;
"pane focus")
  note "focus $4"
  out "$STUB_FOCUS"
  ;;
"pane list")
  out "${STUB_PANE_LIST:?}"
  ;;
"tab list")
  note "tab list $*"
  out "${STUB_TAB_LIST:?}"
  ;;
"tab focus")
  if [ "${STUB_TAB_FOCUS_FAIL:-0}" = "1" ]; then
    exit 1
  fi
  note "tab focus $3"
  out '{}'
  ;;
"workspace list")
  note "workspace list"
  out "${STUB_WORKSPACE_LIST:?}"
  ;;
"workspace focus")
  note "workspace focus $3"
  out '{}'
  ;;
*)
  exit 1
  ;;
esac
