#!/bin/sh
# Wait for a PR's CI to finish, and with --greptile for Greptile's review of the
# head commit, then print one line and exit 0 when everything passed
#
# Run it as a homebased task instead of T3's watch_pull_request, which wakes the
# thread for every bot comment, so the thread wakes once, when the PR is ready to act on
set -eu

usage() {
    echo "usage: watch_pr.sh <pr> [--greptile] [--repo <owner/name>]" >&2
    exit 2
}

[ $# -ge 1 ] || usage
pr=$1
shift

greptile=
repo=
while [ $# -gt 0 ]; do
    case $1 in
    --greptile) greptile=1 ;;
    --repo)
        [ $# -ge 2 ] || usage
        repo=$2
        shift
        ;;
    *) usage ;;
    esac
    shift
done

[ -n "$repo" ] || repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
sha=$(gh pr view "$pr" --repo "$repo" --json headRefOid --jq .headRefOid)
url=$(gh pr view "$pr" --repo "$repo" --json url --jq .url)
short=$(printf %.7s "$sha")

# a push moves the head, which makes the checks and review this run waits on stale
head_moved() {
    now=$(gh pr view "$pr" --repo "$repo" --json headRefOid --jq .headRefOid 2>/dev/null || echo "$sha")
    if [ "$now" != "$sha" ]; then
        echo "PR #$pr: head moved from $short to $(printf %.7s "$now") while waiting; resubmit for the new head $url"
        exit 1
    fi
}

# `gh pr checks --json` needs gh 2.49+, older than some Fleet machines, so read the
# head commit's check runs and commit statuses from statusCheckRollup instead
state_jq='.statusCheckRollup |
    if length == 0 then "none"
    elif any(.[]; (.status // "COMPLETED") != "COMPLETED" or .state == "PENDING" or .state == "EXPECTED") then "pending"
    else "finished" end'
# shellcheck disable=SC2016 # $c is a jq variable, not a shell one
failed_jq='[.statusCheckRollup[] |
    select((.conclusion // .state) as $c | ["FAILURE", "CANCELLED", "TIMED_OUT", "ACTION_REQUIRED", "STARTUP_FAILURE", "ERROR"] | index($c)) |
    (.name // .context)] | join(", ")'

# give up after 3 hours; a failed poll is retried rather than ending the wait.
# Checks take a few seconds to register after a push, so treat the PR as having no
# CI only after 10 minutes without any
ci=
empty=0
for _ in $(seq 360); do
    state=$(gh pr view "$pr" --repo "$repo" --json statusCheckRollup --jq "$state_jq" 2>/dev/null || true)
    if [ "$state" = finished ]; then
        ci=finished
        break
    fi

    if [ "$state" = none ]; then
        empty=$((empty + 1))
        if [ "$empty" -ge 20 ]; then
            ci=none
            break
        fi
    fi

    head_moved
    sleep 30
done

if [ -z "$ci" ]; then
    echo "PR #$pr ($short): CI still pending after 3 hours $url"
    exit 1
fi

failed=$(gh pr view "$pr" --repo "$repo" --json statusCheckRollup --jq "$failed_jq")
if [ "$ci" = none ]; then
    ci_line="no CI checks"
elif [ -n "$failed" ]; then
    ci_line="CI failed ($failed)"
else
    ci_line="CI passed"
fi

if [ -z "$greptile" ]; then
    echo "PR #$pr ($short): $ci_line $url"
    [ -z "$failed" ] && exit 0
    exit 1
fi

# Greptile edits one summary comment per PR, and only its bot account counts, since
# anyone can post the marker on a public repo; its "Last reviewed commit" link names the
# commit the score belongs to. Give the review 1 hour after CI ends
score=
for _ in $(seq 120); do
    score=$(gh api "repos/$repo/issues/$pr/comments" --paginate \
        --jq ".[] | select(.user.login == \"greptile-apps[bot]\") | select(.body | contains(\"<!-- greptile_summary -->\") and contains(\"/commit/$sha\")) | .body" 2>/dev/null |
        sed -n 's/.*<!-- greptile_confidence_score:\([0-9]\) -->.*/\1/p' | tail -n 1)
    [ -n "$score" ] && break
    head_moved
    sleep 30
done

if [ -z "$score" ]; then
    echo "PR #$pr ($short): $ci_line; no Greptile review of $short after 1 hour $url"
    exit 1
fi

echo "PR #$pr ($short): $ci_line; Greptile $score/5 $url"
[ -z "$failed" ] && [ "$score" = 5 ]
