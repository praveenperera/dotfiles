#!/bin/sh
# Wait for the GitHub Actions run of <workflow> on <sha>, then follow it to completion
#
# `gh run list --limit 1` right after a push or tag can return the previous run,
# which makes a deploy install the old version, so match the run to the commit
set -eu

if [ $# -ne 2 ]; then
    echo "usage: watch_run.sh <workflow> <sha>" >&2
    exit 2
fi

workflow=$1
sha=$(git rev-parse "$2")

# a new run takes a few seconds to register; give up after 5 minutes
for _ in $(seq 30); do
    id=$(gh run list --workflow "$workflow" --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId // empty')
    if [ -n "$id" ]; then
        exec gh run watch --exit-status "$id"
    fi

    sleep 10
done

echo "no $workflow run for $sha after 5 minutes" >&2
exit 1
