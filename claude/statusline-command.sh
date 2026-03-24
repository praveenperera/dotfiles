#!/bin/zsh

# Claude Code Status Line - styled after Starship prompt
# Reads JSON from stdin and outputs a formatted status line

input=$(cat)

cwd=$(echo "$input" | jq -r '.cwd // empty')
model_id=$(echo "$input" | jq -r '.model.id // empty')
used_pct=$(echo "$input" | jq -r '.context_window.used_percentage // empty')
output_style=$(echo "$input" | jq -r '.output_style.name // empty')
vim_mode=$(echo "$input" | jq -r '.vim.mode // empty')
agent_name=$(echo "$input" | jq -r '.agent.name // empty')
rl_5h=$(echo "$input" | jq -r '.rate_limits.five_hour.used_percentage // empty')
rl_7d=$(echo "$input" | jq -r '.rate_limits.seven_day.used_percentage // empty')

# ANSI color codes
CYAN='\033[36m'
YELLOW='\033[33m'
BLUE='\033[34m'
GREEN='\033[32m'
MAGENTA='\033[35m'
PURPLE='\033[38;5;141m'
BRIGHT_CYAN='\033[96m'
RED='\033[31m'
RESET='\033[0m'
BOLD='\033[1m'

output=""

# Directory — truncate like Starship (up to 5 components, …/ prefix)
if [[ -n "$cwd" ]]; then
    dir="${cwd/#$HOME/~}"
    IFS='/' read -rA parts <<< "$dir"
    if (( ${#parts[@]} > 6 )); then
        dir="…/${parts[-5]}/${parts[-4]}/${parts[-3]}/${parts[-2]}/${parts[-1]}"
    fi
    output+="${BOLD}${CYAN}${dir}${RESET}"
fi

# Git branch from the working directory
if git -C "$cwd" rev-parse --is-inside-work-tree &>/dev/null 2>&1; then
    branch=$(git --git-dir="$cwd/.git" --work-tree="$cwd" symbolic-ref --short HEAD 2>/dev/null \
             || git --git-dir="$cwd/.git" --work-tree="$cwd" rev-parse --short HEAD 2>/dev/null)
    if [[ -n "$branch" ]]; then
        output+=" ${YELLOW}[${branch}]${RESET}"
    fi
fi

# Model name (blue, simplified)
if [[ -n "$model_id" ]]; then
    display_model="$model_id"
    case "$model_id" in
        *opus*4-6*|*opus*4.6*) display_model="Opus 4.6" ;;
        *opus*)                 display_model="Opus" ;;
        *sonnet*4-5*|*sonnet*4.5*) display_model="Sonnet 4.5" ;;
        *sonnet*)               display_model="Sonnet" ;;
        *haiku*4-5*|*haiku*4.5*) display_model="Haiku 4.5" ;;
        *haiku*)                display_model="Haiku" ;;
    esac
    output+=" ${BOLD}•${RESET} ${BLUE}${display_model}${RESET}"
fi

# Context used (color based on usage)
if [[ -n "$used_pct" ]]; then
    pct_int=$(printf "%.0f" "$used_pct" 2>/dev/null)
    if [[ "$pct_int" =~ ^[0-9]+$ ]]; then
        if (( pct_int < 50 )); then
            ctx_color="$GREEN"
        elif (( pct_int < 80 )); then
            ctx_color="$YELLOW"
        else
            ctx_color="$RED"
        fi
        output+=" ${BOLD}•${RESET} ${ctx_color}ctx: ${pct_int}%${RESET}"
    fi
fi

# Rate limits (13%|45% — 5hr first, 7d after, no prefix)
rl_parts=""
for val in "$rl_5h" "$rl_7d"; do
    if [[ -n "$val" ]]; then
        pct_int=$(printf "%.0f" "$val" 2>/dev/null)
        if [[ "$pct_int" =~ ^[0-9]+$ ]]; then
            if (( pct_int < 50 )); then
                rl_color="$GREEN"
            elif (( pct_int < 80 )); then
                rl_color="$YELLOW"
            else
                rl_color="$RED"
            fi
            if [[ -n "$rl_parts" ]]; then
                rl_parts+="${RESET}|"
            fi
            rl_parts+="${rl_color}${pct_int}%"
        fi
    fi
done
if [[ -n "$rl_parts" ]]; then
    output+=" ${BOLD}•${RESET} usage: ${rl_parts}${RESET}"
fi

# Output style (magenta, only if not default)
if [[ -n "$output_style" && "$output_style" != "default" ]]; then
    output+=" ${BOLD}•${RESET} ${MAGENTA}${output_style}${RESET}"
fi

# Vim mode (purple)
if [[ -n "$vim_mode" ]]; then
    output+=" ${BOLD}•${RESET} ${PURPLE}${vim_mode}${RESET}"
fi

# Agent name (bright cyan)
if [[ -n "$agent_name" ]]; then
    output+=" ${BOLD}•${RESET} ${BRIGHT_CYAN}${agent_name}${RESET}"
fi

printf "%b\n" "$output"
