#!/bin/zsh

# Claude Code Status Line - styled after Starship prompt
# Reads JSON from stdin and outputs a formatted status line

# Read JSON input from stdin
input=$(cat)

# Parse JSON fields using parameter expansion and simple parsing
# Extract values between quotes after the field name
get_field() {
    echo "$input" | grep -o "\"$1\":[^,}]*" | sed 's/.*:"\{0,1\}\([^",}]*\)"\{0,1\}/\1/'
}

cwd=$(get_field "cwd")
git_branch=$(get_field "git_branch")
model=$(get_field "model")
context_pct=$(get_field "context_remaining_percent")
output_style=$(get_field "output_style")
vim_mode=$(get_field "vim_mode")
agent_name=$(get_field "agent_name")

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

# Build output
output=""

# Directory (truncated like Starship, cyan)
if [[ -n "$cwd" ]]; then
    # Replace home with ~
    dir="${cwd/#$HOME/~}"

    # Truncate to last 3 components with …/ prefix if needed
    IFS='/' read -rA parts <<< "$dir"
    if (( ${#parts[@]} > 4 )); then
        dir="…/${parts[-3]}/${parts[-2]}/${parts[-1]}"
    fi

    output+="${BOLD}${CYAN}${dir}${RESET}"
fi

# Git branch (yellow)
if [[ -n "$git_branch" ]]; then
    output+=" ${YELLOW}[${git_branch}]${RESET}"
fi

# Model name (blue, simplified)
if [[ -n "$model" ]]; then
    # Simplify model names
    display_model="$model"
    case "$model" in
        *opus*4-6*|*opus*4.6*) display_model="Opus 4.6" ;;
        *opus*|*Opus*) display_model="Opus 4.5" ;;
        *sonnet*4-5*|*sonnet*4.5*) display_model="Sonnet 4.5" ;;
        *sonnet*|*Sonnet*) display_model="Sonnet 4" ;;
        *haiku*4-5*|*haiku*4.5*) display_model="Haiku 4.5" ;;
        *haiku*|*Haiku*) display_model="Haiku" ;;
    esac
    output+=" ${BOLD}•${RESET} ${BLUE}${display_model}${RESET}"
fi

# Context remaining (color based on amount)
if [[ -n "$context_pct" && "$context_pct" =~ ^[0-9]+$ ]]; then
    if (( context_pct > 50 )); then
        ctx_color="$GREEN"
    elif (( context_pct > 20 )); then
        ctx_color="$YELLOW"
    else
        ctx_color="$RED"
    fi
    output+=" ${BOLD}•${RESET} ${ctx_color}Context: ${context_pct}%${RESET}"
fi

# Output style (magenta, only if not default)
if [[ -n "$output_style" && "$output_style" != "default" && "$output_style" != "normal" ]]; then
    output+=" ${BOLD}•${RESET} ${MAGENTA}${output_style}${RESET}"
fi

# Vim mode (purple)
if [[ "$vim_mode" == "true" ]]; then
    output+=" ${BOLD}•${RESET} ${PURPLE}vim${RESET}"
fi

# Agent name (bright cyan)
if [[ -n "$agent_name" ]]; then
    output+=" ${BOLD}•${RESET} ${BRIGHT_CYAN}⚡${agent_name}${RESET}"
fi

echo -e "$output"
