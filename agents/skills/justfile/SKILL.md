---
name: justfile
description: |
  Guide for creating, editing, and understanding justfiles - the command runner inspired by make.
  Use when: (1) Creating a new justfile for a project, (2) Adding or modifying recipes in an existing justfile,
  (3) Understanding justfile syntax including recipes, variables, settings, attributes, and modules,
  (4) Debugging justfile issues, (5) User mentions "just", "justfile", or asks about running project commands.
---

# Justfile

A justfile stores project-specific commands (recipes) that can be run with `just <recipe>`.

## Quick Reference

```just
# Recipe with doc comment
recipe-name:
    echo "commands are indented with 4 spaces or 1 tab"

# Recipe with parameters
greet name="world":
    echo "Hello, {{name}}!"

# Recipe with dependencies
build: clean compile
    echo "Built!"

# Shebang recipe for multi-line scripts
test:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo test
```

## Core Syntax

### Recipes

```just
# Basic recipe
build:
    cargo build

# With parameters (with defaults)
test target="all" flags="":
    cargo test {{target}} {{flags}}

# Variadic: + = one or more, * = zero or more
backup +FILES:
    cp {{FILES}} /backup/

deploy *FLAGS:
    ./deploy.sh {{FLAGS}}

# Dependencies run before recipe
test: build
    ./test

# Sequential dependencies with &&
release: build && deploy notify
```

### Variables

```just
version := "1.0.0"
build_dir := "target"

# Backtick captures command output
git_hash := `git rev-parse --short HEAD`

# Environment variable access
home := env('HOME')
editor := env('EDITOR', 'vim')  # with default
```

### Settings

```just
set shell := ["bash", "-uc"]
set dotenv-load                    # load .env file
set positional-arguments           # pass args as $1, $2
set export                         # export all vars as env vars
set working-directory := "subdir"  # change working dir
set quiet                          # suppress command echo
```

### Attributes

```just
[group('build')]                   # group in --list output
[private]                          # hide from --list
[no-cd]                            # don't change directory
[confirm("Delete all?")]           # require confirmation
[working-directory: 'subdir']      # per-recipe working dir
[script('bash')]                   # run as bash script
[linux]                            # only run on Linux
[macos]                            # only run on macOS
[windows]                          # only run on Windows
[unix]                             # run on any Unix
[default]                          # default recipe
```

### Aliases

```just
alias b := build
alias t := test

[private]
alias ba := build-android
```

### Conditionals

```just
foo := if "x" == "x" { "yes" } else { "no" }
bar := if env('CI', '') != '' { "ci" } else { "local" }

# Regex matching
baz := if "hello" =~ 'hel+o' { "match" } else { "no" }
```

### String Types

```just
single := 'no escapes: \n stays as \n'
double := "escapes work: \n is newline"
indented := '''
    lines are unindented
    based on common prefix
'''
shell_expanded := x'~/{{$USER}}'   # expands ~ and $VAR
format := f'Hello {{name}}!'       # format string
```

## Command Modifiers

```just
recipe:
    @echo "@ suppresses echo of this line"
    -failing-command  # - ignores non-zero exit
    echo "normal line is echoed then run"
```

## Functions

### System Info
- `os()` - "linux", "macos", "windows"
- `arch()` - "x86_64", "aarch64"
- `num_cpus()` - CPU count

### Paths
- `justfile()` - path to current justfile
- `justfile_directory()` - dir containing justfile
- `invocation_directory()` - where just was called from
- `home_directory()` - user's home dir

### Strings
- `quote(s)` - shell-escape string
- `replace(s, from, to)` - replace substring
- `trim(s)` - remove whitespace
- `uppercase(s)` / `lowercase(s)`

### External
- `shell(cmd, args...)` - run shell command
- `env('VAR')` - get env var (errors if missing)
- `env('VAR', 'default')` - get env var with default

## Modules and Imports

```just
# Import another justfile (recipes merged)
import 'common.just'
import? 'optional.just'  # optional import

# Module (namespaced recipes)
mod deploy              # looks for deploy.just or deploy/mod.just
mod? tools 'ci.just'    # optional, custom path

# Call module recipe
# just deploy::production
# just deploy production  (subcommand style)
```

## Recommended Structure

Organize with groups and sections:

```just
[default]
list:
    @just --list

# ------------------------------------------------------------------------------
# build
# ------------------------------------------------------------------------------

[group('build')]
build:
    cargo build

[group('build')]
build-release:
    cargo build --release

[private]
alias br := build-release

# ------------------------------------------------------------------------------
# test
# ------------------------------------------------------------------------------

[group('test')]
test *args="":
    cargo test {{args}}

[group('test')]
[working-directory: 'tests']
integration:
    ./run-integration.sh
```

## Common Patterns

### Default recipe lists available commands
```just
[default]
list:
    @just --list
```

### CI recipe chains multiple checks
```just
[group('ci')]
[script('bash')]
ci:
    set -e
    just fmt --check
    just lint
    just test
```

### Confirmation for dangerous operations
```just
[confirm("Delete all build artifacts?")]
[group('util')]
clean:
    rm -rf target/ build/ dist/
```

### Cross-platform recipes
```just
[linux]
open path:
    xdg-open {{path}}

[macos]
open path:
    open {{path}}

[windows]
open path:
    start {{path}}
```

### Notification helpers
```just
[private]
notify msg:
    @say {{msg}} 2>/dev/null || echo {{msg}}
```

## Running Just

```sh
just              # run default recipe
just recipe       # run specific recipe
just recipe arg   # with argument
just -l           # list recipes
just --summary    # compact list
just -n recipe    # dry run
just --fmt        # format justfile (unstable)
just mod::recipe  # run recipe in module
```
