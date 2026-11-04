#!/usr/bin/env bash
# Runs a test command under Metal's API and shader validation layers and fails when either
# reports an error, even if the command exits 0. Repo-agnostic: pass the repo's own kernel
# and stage test command. Needs the GPU, so run it as a step of a homebased queue job
#
#   metal_validate.sh [-o LOG] -- CMD [ARGS...]
#
# Checks the command's output and the unified log for the run's window. The match patterns
# are a first cut that has not been confirmed against real validation output; when a run
# shows a validation error these miss, add its wording here
set -uo pipefail
log=$(mktemp -t metal-validate)
if [[ ${1:-} == -o ]]; then log=$2; shift 2; fi
[[ ${1:-} == -- ]] && shift
(($#)) || { echo "usage: metal_validate.sh [-o LOG] -- CMD [ARGS...]" >&2; exit 2; }

pattern='failed assertion|Invalid (device|threadgroup|constant|texture) (load|store|read|write)|shader validation|validation (error|failed)|GPU Address Fault|MTLCommandBufferError'
start=$(date '+%Y-%m-%d %H:%M:%S')

MTL_DEBUG_LAYER=1 MTL_SHADER_VALIDATION=1 "$@" >"$log" 2>&1
status=$?

# shader validation may report through os_log rather than stderr, so read both
syslog=$(log show --start "$start" --style compact \
  --predicate '(subsystem BEGINSWITH "com.apple.Metal" OR eventMessage CONTAINS[c] "shader validation") AND process != "log"' 2>/dev/null)

found=0
if grep -Ei "$pattern" "$log"; then found=1; fi
if grep -Ei "$pattern" <<<"$syslog"; then found=1; fi

echo "output: $log"
if ((status != 0)); then
  echo "FAIL: command exited $status"
  exit "$status"
fi
if ((found)); then
  echo "FAIL: validation errors above"
  exit 1
fi
echo "PASS: no validation errors"
