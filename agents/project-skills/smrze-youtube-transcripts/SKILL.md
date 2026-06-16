---
name: smrze-youtube-transcripts
description: Get transcripts from YouTube videos using `smrze t` / `smrze transcript`. Use when the user asks to fetch, create, save, inspect, or summarize a transcript from a YouTube video URL with smrze.
---

# SMRZE YouTube Transcripts

## Overview

Use `smrze t` to generate a transcript from a YouTube URL. Always put the YouTube video link in quotes when passing it to the command.

## Quick Start

```sh
smrze t --quiet --no-timestamps "https://www.youtube.com/watch?v=VIDEO_ID"
```

Equivalent long form:

```sh
smrze transcript --quiet --no-timestamps "https://www.youtube.com/watch?v=VIDEO_ID"
```

The quoted URL requirement matters for YouTube links because shells interpret characters such as `&`, `?`, and `=`. Do not run `smrze t https://...` unquoted.

## Save a Transcript

When the transcript should be written to disk, pass an output directory. `smrze` writes `transcript.txt` inside that directory.

```sh
mkdir -p _scratch/youtube-transcripts
smrze t --quiet --no-timestamps --output _scratch/youtube-transcripts "https://www.youtube.com/watch?v=VIDEO_ID"
```

For this user's projects, put ad hoc transcript artifacts under the repo-root `_scratch/` directory unless the user asks for a different location.

## Options

- Use `--force` when the user explicitly wants to recompute instead of reading cached artifacts.
- Use `--no-timestamps` for normal transcript extraction so the output is plain transcript text without timestamps or speaker diarization.
- Use `--format json` when downstream processing needs structured transcript data.
- Use `--mode word` only when word-level output is useful; otherwise use the default transcript mode.
- Use `--open` only with `--output` when the user wants the written transcript opened locally.

## Recommended Flow

1. Confirm the input is a YouTube URL.
2. Quote the full URL in the `smrze t` command.
3. Include `--no-timestamps` unless the user wants timestamps or speaker diarization.
4. Use stdout for quick inspection or `--output` for saved transcripts.
5. After a saved run, read the generated `transcript.txt` before summarizing or quoting it.
