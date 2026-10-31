#!/usr/bin/env bash
# Compares two PTX builds kernel by kernel: calls left in the PTX, ptxas registers, stack
# and spills, SASS instruction count, and the opcodes that flag known codegen traps
# (generic LD where LDG/LDS belong, FSETP+FSEL where FMNMX belongs, local LDL/STL). Needs
# a CUDA toolkit, no GPU
#
#   sass_mix.sh REF.ptx NEW.ptx [ARCH] [KERNEL...]
#
# ARCH defaults to sm_120; with no kernels it lists every entry with a changed count
set -euo pipefail
ref=$1 new=$2 arch=${3:-sm_120}
shift $(($# < 3 ? $# : 3))
bin=$(dirname "$(command -v ptxas || echo /usr/local/cuda/bin/ptxas)")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

for v in ref new; do
  ptx=${!v}
  echo "$v: $(grep -cE 'call\.uni|^\s*call\b' "$ptx" || true) calls in $(basename "$ptx")"
  "$bin/ptxas" -arch="$arch" -O3 -v "$ptx" -o "$tmp/$v.cubin" 2> "$tmp/$v.log"
  # one line per kernel: name regs stack spill_st spill_ld
  awk '/Compiling entry function/ { gsub(/\x27/, "", $7); k = $7 }
       /bytes stack frame/ { st = $1; ss = $5; sl = $9 }
       /Used [0-9]+ registers/ { for (i = 1; i <= NF; i++) if ($i == "Used") r = $(i + 1); print k, r, st, ss, sl }' \
    "$tmp/$v.log" | sort > "$tmp/$v.res"
done

if [[ $# -eq 0 ]]; then
  mapfile -t kernels < <(join "$tmp/ref.res" "$tmp/new.res" | awk '$2 != $6 || $3 != $7 || $4 != $8' | cut -d' ' -f1)
  echo "${#kernels[@]} kernels with changed registers, stack or spills"
else
  kernels=("$@")
fi

for k in "${kernels[@]}"; do
  echo "== $k"
  for v in ref new; do
    res=$(awk -v k="$k" '$1 == k { printf "regs %s stack %s spill %s/%s", $2, $3, $4, $5 }' "$tmp/$v.res")
    # a kernel new in one build (or renamed) has no counterpart to compare
    if [[ -z $res ]]; then
      printf '%-4s not in this build\n' "$v"
      continue
    fi
    "$bin/cuobjdump" -sass -fun "$k" "$tmp/$v.cubin" > "$tmp/$v.sass" 2>/dev/null || true
    ops=$(grep -oE '/\*[0-9a-f]{4}\*/\s+(@!?U?P[0-9T] )?[A-Z0-9_]+' "$tmp/$v.sass" | awk '{print $NF}' |
      awk '$1 ~ /^(LD|LDG|LDS|LDL|STL|FSETP|FSEL|FMNMX|HMMA|MUFU|CALL)$/' | sort | uniq -c | awk '{printf "%s %s  ", $2, $1}')
    printf '%-4s %-36s insts %5s  %s\n' "$v" "$res" "$(grep -cE '/\*[0-9a-f]{4}\*/' "$tmp/$v.sass")" "$ops"
  done
done
