#!/usr/bin/env bash
# runorfocus for Umbriel: launch the app or switch/cycle through its windows.
set -euo pipefail

TAG=""
APPID=""
WINDOW_TITLE=""
IGNORE_TITLE=""
COMMAND=""
CMD=()
NO_FOCUSLAST=0
STATE_DIR="${TMPDIR:-/tmp}/runorfocus-state"
FOCUS_CHECK_DELAY=0.05
FOCUS_CHECK_MAX=10
SEP=$'\037'

# Sends an action to the Umbriel compositor.
um() {
  umbriel msg "$1" >/dev/null 2>&1
}

# Lists windows as JSON (array of objects).
get_windows() {
  umbriel windows --json
}

get_active_workspace() {
  umbriel workspaces --json | jq -r 'first(.[] | select(.active == true) | .id) // empty'
}

# Filters windows by app_id / title and sorts by id.
matching_windows_jq() {
  jq -c \
    --arg appid "$APPID" \
    --arg window_title "$WINDOW_TITLE" \
    --arg ignore_title "$IGNORE_TITLE" '
    def wins: if type == "array" then . else (.windows // []) end;

    [ wins[]
      | select(
          ($appid == "" or (.app_id // "") == $appid)
          and
          ((.scratchpad // "") == "")
          and
          ($window_title == ""
            or ((.title // "") | ascii_downcase | contains($window_title | ascii_downcase)))
          and
          ($ignore_title == ""
            or ((.title // "") | ascii_downcase | contains($ignore_title | ascii_downcase) | not))
        )
    ]
    | sort_by(.id)'
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    -t|--tag)
      [ "$#" -ge 2 ] || exit 1
      TAG="$2"
      shift 2
      ;;
    -T|--title)
      [ "$#" -ge 2 ] || exit 1
      WINDOW_TITLE="$2"
      shift 2
      ;;
    -a|--appid)
      [ "$#" -ge 2 ] || exit 1
      APPID="$2"
      shift 2
      ;;
    -i|--ignore)
      [ "$#" -ge 2 ] || exit 1
      IGNORE_TITLE="$2"
      shift 2
      ;;
    -c|--command)
      [ "$#" -ge 2 ] || exit 1
      COMMAND="$2"
      shift 2
      ;;
    -n|--no-focuslast)
      NO_FOCUSLAST=1
      shift
      ;;
    --)
      shift
      CMD=("$@")
      break
      ;;
    *)
      exit 1
      ;;
  esac
done

wait_for_window() {
  local max_checks=15
  local check=0
  local opened_count

  while [ "$check" -lt "$max_checks" ]; do
    opened_count=$(get_windows | matching_windows_jq | jq -r 'length') || opened_count=0

    if [ "$opened_count" -gt 0 ]; then
      return 0
    fi

    sleep 0.2
    check=$((check + 1))
  done

  return 1
}

get_state_file() {
  local state_key
  state_key=$(printf '%s\037%s\037%s\n' "$APPID" "$WINDOW_TITLE" "$IGNORE_TITLE" | cksum | awk '{print $1}')
  printf '%s/%s.last-id\n' "$STATE_DIR" "$state_key"
}

read_last_id() {
  local state_file
  state_file=$(get_state_file)

  if [ -f "$state_file" ]; then
    cat "$state_file"
  fi
}

write_last_id() {
  local state_file
  state_file=$(get_state_file)
  mkdir -p "$STATE_DIR"
  printf '%s\n' "$1" > "$state_file"
}

get_focused_id() {
  local active_workspace
  active_workspace=$(get_active_workspace) || return 0
  [ -n "$active_workspace" ] || return 0

  get_windows | jq -r --arg active_workspace "$active_workspace" '
    def wins: if type == "array" then . else (.windows // []) end;
    first(wins[] | select(.focused == true and .workspace == $active_workspace) | .id) // empty' || true
}

focus_window_at_idx() {
  local idx="$1"
  local target_id="${ids[$idx]}"
  local ws="${workspaces[$idx]}"
  local active_workspace
  local focused_id=""
  local check=0

  # The workspace field is in the form "OUTPUT:NAME" (e.g. "DP-5:1"),
  # and the action expects "<workspace>/<output>".
  active_workspace=$(get_active_workspace) || active_workspace=""
  if [[ "$ws" == *:* && "$ws" != "$active_workspace" ]]; then
    um "workspace-switch:${ws##*:}/${ws%:*}" || true
  fi
  um "window-focus:$target_id" || true

  while [ "$check" -lt "$FOCUS_CHECK_MAX" ]; do
    focused_id="$(get_focused_id)"
    if [ "$focused_id" = "$target_id" ]; then
      write_last_id "$target_id"
      return 0
    fi

    sleep "$FOCUS_CHECK_DELAY"
    check=$((check + 1))
  done

  return 1
}

selected_windows=$(get_windows | matching_windows_jq)

ids=(); workspaces=()
while IFS="$SEP" read -r w_id w_ws; do
  [ -n "$w_id" ] || continue
  ids+=("$w_id")
  workspaces+=("$w_ws")
done < <(
  echo "$selected_windows" |
    jq -r --arg sep "$SEP" '.[] | [.id, (.workspace // "")] | join($sep)'
)

matching_count=${#ids[@]}
last_id=$(read_last_id)
focused_id=$(get_focused_id || true)

# no matching windows
if [ "$matching_count" -eq 0 ]; then
  [ "${#CMD[@]}" -gt 0 ] || exit 1
  if [ -n "$TAG" ]; then
    um "workspace-switch:$TAG" || true
  fi
  env "${CMD[@]}" &
  if [ -n "$COMMAND" ] && wait_for_window; then
    um "$COMMAND" || true
  fi
  exit 0
fi

# one matching window
if [ "$matching_count" -eq 1 ]; then
  if [ "${focused_id:-}" = "${ids[0]}" ]; then
    write_last_id "${ids[0]}"
    if [ "$NO_FOCUSLAST" -ne 1 ]; then
      um "window-focus-last" || true
    fi
    exit 0
  else
    focus_window_at_idx 0 || true
    exit 0
  fi
fi

# more than one window: cycle through them
focus_idx=-1
remembered_idx=-1
for i in "${!ids[@]}"; do
  if [ "${ids[$i]}" = "${focused_id:-}" ]; then
    focus_idx=$i
    break
  fi
done

if [ -n "$last_id" ]; then
  for i in "${!ids[@]}"; do
    if [ "${ids[$i]}" = "$last_id" ]; then
      remembered_idx=$i
      break
    fi
  done
fi

if [ "$focus_idx" -ge 0 ]; then
  next_idx=$(( (focus_idx + 1) % matching_count ))
elif [ "$remembered_idx" -ge 0 ]; then
  next_idx=$(( (remembered_idx + 1) % matching_count ))
else
  next_idx=0
fi

focus_window_at_idx "$next_idx" || true

exit 0
