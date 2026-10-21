#!/bin/sh
# Wait for the GitHub Actions run of <workflow> on <sha> and print one line when it ends
#
# `gh run list --limit 1` right after a push or tag can return the previous run,
# which makes a deploy install the old version, so match the run to the commit
#
# Stays silent while the run is in progress: a Monitor wakes the agent for every
# stdout line, and `gh run watch` redraws its job list every few seconds
set -eu

if [ $# -ne 2 ]; then
    echo "usage: watch_run.sh <workflow> <sha>" >&2
    exit 2
fi

workflow=$1
sha=$(git rev-parse "$2")

# a new run takes a few seconds to register; give up after 5 minutes
id=
for _ in $(seq 30); do
    id=$(gh run list --workflow "$workflow" --commit "$sha" --limit 1 --json databaseId --jq '.[0].databaseId // empty' 2>/dev/null || true)
    [ -n "$id" ] && break
    sleep 10
done

if [ -z "$id" ]; then
    echo "$workflow: no run for $sha after 5 minutes"
    exit 1
fi

# give up after 90 minutes; a failed poll is retried rather than ending the wait
for _ in $(seq 270); do
    status=$(gh run view "$id" --json status --jq .status 2>/dev/null || true)
    [ "$status" = completed ] && break
    sleep 20
done

if [ "$status" != completed ]; then
    echo "$workflow run $id: still ${status:-unknown} after 90 minutes"
    exit 1
fi

conclusion=$(gh run view "$id" --json conclusion --jq .conclusion)
url=$(gh run view "$id" --json url --jq .url)
if [ "$conclusion" = success ]; then
    echo "$workflow run $id: success $url"
    exit 0
fi

failed=$(gh run view "$id" --json jobs --jq '[.jobs[] | select(.conclusion != "success" and .conclusion != "skipped") | .name] | join(", ")')
echo "$workflow run $id: $conclusion ($failed) $url"
exit 1
