#!/usr/bin/env bash
# End-to-end test of the pass plugin's Type and Copy on a live Hyprland session.
#
#   nix-shell --run 'cargo build --release'
#   nix-shell --run tests/e2e/pass_plugin.sh [path/to/vonal]
#
# It runs its own daemon with a throwaway HOME, socket, GPG key and password
# store, so a running vonal and the real password store aren't touched. Copy
# uses the primary selection, so the clipboard isn't touched either.
#
# Windows open and take focus while it runs, so don't type until it's finished.
# Keys are only sent after checking which window is focused.
# The passphrase prompt comes from the system's /etc/gnupg/gpg-agent.conf and
# has to be a graphical pinentry, which this script answers with real key presses.
set -u

repo=$(cd "$(dirname "$0")/../.." && pwd)
VONAL=${1:-$repo/target/release/vonal}
VONALC=${VONALC:-$repo/target/release/vonalc}
PASSPHRASE=throwaway-passphrase
SECRET=dummy-secret

REAL_RUNTIME_DIR=$XDG_RUNTIME_DIR
case "$WAYLAND_DISPLAY" in
    /*) WAYLAND_SOCKET_PATH=$WAYLAND_DISPLAY ;;
    *) WAYLAND_SOCKET_PATH=$REAL_RUNTIME_DIR/$WAYLAND_DISPLAY ;;
esac
# vonal's own libraries shouldn't leak into the tools under test
VONAL_LD_LIBRARY_PATH=${LD_LIBRARY_PATH:-}
unset LD_LIBRARY_PATH
REAL_AGENT_SOCKET=$(gpgconf --list-dirs agent-socket)

T=$(mktemp -d)
# Inside the real runtime dir, because a socket path has a short length limit
TEST_RUNTIME_DIR=$(mktemp -d -p "$REAL_RUNTIME_DIR" vonal-e2e.XXXXXX)
export HOME=$T/home
export XDG_RUNTIME_DIR=$TEST_RUNTIME_DIR
export WAYLAND_DISPLAY=$WAYLAND_SOCKET_PATH
# Must not be $HOME/.gnupg: GnuPG treats that as the default home and connects
# to the user's real agent, which would store the test keys in the real keyring.
export GNUPGHOME=$T/gnupg
export PASSWORD_STORE_DIR=$HOME/.password-store
export PASSWORD_STORE_X_SELECTION=primary
export PASSWORD_STORE_CLIP_TIME=4
unset GPG_TTY
mkdir -p "$HOME" "$GNUPGHOME"
chmod 700 "$GNUPGHOME"
LOG=$T/vonal.log

if [ "$(gpgconf --list-dirs agent-socket)" = "$REAL_AGENT_SOCKET" ]; then
    echo "Refusing to run: gpg would use the real agent at $REAL_AGENT_SOCKET" >&2
    rm -rf "$T" "$TEST_RUNTIME_DIR"
    exit 2
fi

passed=0
failed=0
ok() { echo "  ok    $1"; passed=$((passed + 1)); }
bad() { echo "  FAIL  $1"; failed=$((failed + 1)); }
check() { if eval "$2"; then ok "$1"; else bad "$1"; fi; }

hypr() { XDG_RUNTIME_DIR=$REAL_RUNTIME_DIR hyprctl "$@"; }
active_pid() { hypr activewindow -j | sed -n 's/.*"pid": \([0-9]*\).*/\1/p'; }
is_mapped() { hypr clients -j | grep -q "\"pid\": $1,"; }
is_pinentry() { readlink "/proc/$1/exe" 2>/dev/null | grep -q pinentry; }
is_event_loop_blocked() { grep -q pipe "/proc/$1/task/$1/wchan"; }

wait_until() { # timeout_in_tenths condition...
    local tries=$1
    shift
    while ((tries-- > 0)); do
        "$@" && return 0
        sleep 0.1
    done
    return 1
}
has_focus() { [ "$(active_pid)" = "$1" ]; }
pinentry_has_focus() { local pid; pid=$(active_pid); [ -n "$pid" ] && is_pinentry "$pid"; }
is_hidden() { ! is_mapped "$1"; }
log_contains() { grep -q -- "$1" "$LOG"; }
primary_is_secret() { [ "$(timeout 2 wl-paste --primary --no-newline 2>/dev/null)" = "$SECRET" ]; }
file_has_content() { [ -s "$1" ]; }

# Never sends anything to a window that isn't the expected one
send_to() { # pid wtype_args...
    local pid=$1
    shift
    if wait_until 50 has_focus "$pid"; then
        sleep 0.2
        wtype "$@"
    else
        echo "  (keys not sent: pid $pid has no focus, focused: $(active_pid))"
        return 1
    fi
}
answer_prompt() { # wtype_args...
    if wait_until 100 pinentry_has_focus; then
        sleep 0.3
        wtype "$@"
    else
        echo "  (no passphrase prompt got focus, focused: $(active_pid))"
        return 1
    fi
}

forget_passphrase() {
    gpgconf --kill gpg-agent
    sleep 0.5
}
remember_passphrase() {
    gpg-connect-agent /bye >/dev/null 2>&1
    local grip
    for grip in $(gpg --list-secret-keys --with-keygrip --with-colons | awk -F: '/^grp/{print $10}'); do
        "$(gpgconf --list-dirs libexecdir)/gpg-preset-passphrase" --preset -P "$PASSPHRASE" "$grip"
    done
}

open_vonal() {
    "$VONALC" set_query 'pass dummy' show
    wait_until 50 has_focus "$VONAL_PID"
}

# Opens a terminal that saves the line it receives, like a password field
open_target() {
    TARGET_FILE=$T/typed-$RANDOM
    foot --title vonal-e2e-target sh -c "IFS= read -r line; printf %s \"\$line\" > '$TARGET_FILE'" &
    TARGET_PID=$!
    wait_until 50 has_focus "$TARGET_PID"
}
close_target() {
    kill "$TARGET_PID" 2>/dev/null
    wait "$TARGET_PID" 2>/dev/null
}
# The typing has no end signal, so the Enter comes after a pause
finish_typing() {
    sleep 1.5
    send_to "$TARGET_PID" -k Return
    wait_until 30 file_has_content "$TARGET_FILE"
}
typed_is_secret() { [ "$(cat "$TARGET_FILE" 2>/dev/null)" = "$SECRET" ]; }

cleanup() {
    kill "${VONAL_PID:-}" "${TARGET_PID:-}" 2>/dev/null
    gpgconf --kill gpg-agent 2>/dev/null
    gpgconf --remove-socketdir 2>/dev/null
    rm -rf "$T" "$TEST_RUNTIME_DIR"
}
trap cleanup EXIT

echo "binary: $VONAL"
gpg --batch --quiet --pinentry-mode loopback --passphrase "$PASSPHRASE" \
    --quick-generate-key 'vonal e2e <e2e@invalid>' default default never 2>/dev/null
echo allow-preset-passphrase >"$GNUPGHOME/gpg-agent.conf"
fingerprint=$(gpg --list-keys --with-colons | awk -F: '/^fpr/{print $10; exit}')
pass init "$fingerprint" >/dev/null
printf '%s\nsecond line\n' "$SECRET" | pass insert -m -f dummy >/dev/null

LD_LIBRARY_PATH=$VONAL_LD_LIBRARY_PATH "$VONAL" >"$LOG" 2>&1 </dev/null &
VONAL_PID=$!
if ! wait_until 50 log_contains 'waiting for clients'; then
    bad 'daemon starts'
    cat "$LOG"
    exit 1
fi
ok 'daemon starts'

echo 'Type, passphrase cached'
forget_passphrase
remember_passphrase
open_target
open_vonal
send_to "$VONAL_PID" -k Return
check 'window hides' 'wait_until 20 is_hidden $VONAL_PID'
finish_typing
check 'types only the first line into the previous window' typed_is_secret
close_target

echo 'Type, passphrase prompt'
forget_passphrase
open_target
open_vonal
send_to "$VONAL_PID" -k Return
check 'window hides before the prompt is answered' 'wait_until 20 is_hidden $VONAL_PID'
answer_prompt -- "$PASSPHRASE" && wtype -k Return
finish_typing
check 'types into the previous window after the prompt' typed_is_secret
close_target

echo 'Copy, passphrase cached'
forget_passphrase
remember_passphrase
open_vonal
send_to "$VONAL_PID" -k Right -k Return
check 'window hides' 'wait_until 20 is_hidden $VONAL_PID'
check 'copies the secret' 'wait_until 30 primary_is_secret'
sleep 5 # lets pass restore the selection

echo 'Copy, passphrase prompt'
forget_passphrase
open_vonal
send_to "$VONAL_PID" -k Right -k Return
check 'window hides before the prompt is answered' 'wait_until 20 is_hidden $VONAL_PID'
wait_until 100 pinentry_has_focus
check 'event loop is not blocked during the prompt' '! is_event_loop_blocked $VONAL_PID'
answer_prompt -- "$PASSPHRASE" && wtype -k Return
check 'copies the secret after the prompt' 'wait_until 50 primary_is_secret'
sleep 5

echo 'Copy, prompt cancelled'
forget_passphrase
open_vonal
send_to "$VONAL_PID" -k Right -k Return
answer_prompt -k Escape
check 'logs the error' 'wait_until 50 log_contains "pass plugin: .*\(decrypt\|cancel\|fail\)"'
check 'daemon still works' 'open_vonal'
"$VONALC" hide

echo
echo "$passed passed, $failed failed"
if ((failed > 0)); then
    echo '--- daemon log'
    cut -c1-200 "$LOG"
fi
((failed == 0))
