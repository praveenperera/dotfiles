# Link sub-issues to parent issues

Use this guide when the user wants to attach an existing issue as a sub-issue of
another issue in GitHub.

## When to use `gh api`

Use `gh api` for sub-issue relationships. There is no regular `gh issue` subcommand
for this flow, and connector tools may not expose it either.

The REST endpoint is:

```bash
POST /repos/{owner}/{repo}/issues/{issue_number}/sub_issues
```

The request body must include `sub_issue_id`, which is the REST issue ID for the
child issue, not the `#123` issue number.

## Prerequisites

1. The local `gh` CLI is installed
2. `gh auth status` succeeds for the target host
3. You know:
   - the parent issue number
   - the child issue number
   - the repository name in `owner/repo` form

## Step 1: get the child issue REST ID

The sub-issues API expects the child issue's numeric REST ID. Fetch it first:

```bash
gh api repos/OWNER/REPO/issues/CHILD_ISSUE_NUMBER --jq '.id'
```

Example:

```bash
child_id="$(gh api repos/bitcoinppl/cove/issues/169 --jq '.id')"
echo "$child_id"
```

If you already have a full issue JSON payload from another command, reuse `.id` from
that output instead of fetching it again.

## Step 2: attach the child to the parent

Call the sub-issues endpoint on the parent issue:

```bash
gh api -X POST repos/OWNER/REPO/issues/PARENT_ISSUE_NUMBER/sub_issues \
  -F sub_issue_id="$child_id"
```

Concrete example:

```bash
child_id="$(gh api repos/bitcoinppl/cove/issues/169 --jq '.id')"

gh api -X POST repos/bitcoinppl/cove/issues/168/sub_issues \
  -F sub_issue_id="$child_id"
```

## Step 3: verify the relationship

List the parent issue's sub-issues:

```bash
gh api repos/OWNER/REPO/issues/PARENT_ISSUE_NUMBER/sub_issues
```

Useful filtered check:

```bash
gh api repos/OWNER/REPO/issues/PARENT_ISSUE_NUMBER/sub_issues \
  --jq '.[] | {number, id, title}'
```

To check the reverse direction for the child issue:

```bash
gh api repos/OWNER/REPO/issues/CHILD_ISSUE_NUMBER/parent \
  --jq '{number, id, title}'
```

## Important caveat: use `-F`, not `-f`

Use `-F sub_issue_id=...`, not `-f sub_issue_id=...`.

- `-F/--field` performs typed conversion in `gh api`
- `-f/--raw-field` sends strings
- GitHub expects `sub_issue_id` to be an integer

If you send the value as a string, GitHub can reject the request with `422
Unprocessable Entity`.

## Common failure modes

### `422 Unprocessable Entity`

Usually one of these:

- `sub_issue_id` was sent with `-f` instead of `-F`
- you passed the issue number instead of the REST issue ID
- the relationship is not allowed by GitHub for that pair of issues
- the sub-issue relationship already exists

Re-check the child ID with:

```bash
gh api repos/OWNER/REPO/issues/CHILD_ISSUE_NUMBER --jq '{number, id, title}'
```

### `404 Not Found`

Usually one of these:

- the repo path is wrong
- your token cannot access the repo
- the issue number does not exist

Check auth and visibility first:

```bash
gh auth status
gh repo view OWNER/REPO
```

### `403 Forbidden`

The token is authenticated but lacks permission for that repository or issue action.

## Raw curl shape

Use this only when `gh` is unavailable. The `gh api` form is usually simpler because
it reuses local auth.

```bash
curl -L \
  -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer $GITHUB_TOKEN" \
  -H "X-GitHub-Api-Version: 2026-03-10" \
  "https://api.github.com/repos/OWNER/REPO/issues/PARENT_ISSUE_NUMBER/sub_issues" \
  -d "{\"sub_issue_id\": $child_id}"
```

## Source

Official docs:
[GitHub REST API sub-issues](https://docs.github.com/en/rest/issues/sub-issues)
