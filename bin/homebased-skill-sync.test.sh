#!/usr/bin/env bash
# Checks homebased-skill-sync against a local bare repo and a stub homebased;
# it touches no real clone and removes its temporary files
set -Eeuo pipefail

SCRIPT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/homebased-skill-sync"
readonly SCRIPT

WORK="$(mktemp -d)"
readonly WORK
trap 'rm -rf -- "${WORK}"' EXIT

readonly REMOTE="${WORK}/remote.git"
readonly CLONE="${WORK}/share/skill-src"
readonly STUBS="${WORK}/stubs"
readonly SKILL_FILE="${CLONE}/.agents/skills/homebased/SKILL.md"

# the user's global git config can require signing or rewrite URLs
export GIT_CONFIG_GLOBAL=/dev/null
export GIT_CONFIG_NOSYSTEM=1

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

# keep git and the shell but drop any real homebased from PATH
isolated_path() {
    local dir
    local path=
    local rest="${PATH}:"

    while [[ -n "${rest}" ]]; do
        dir="${rest%%:*}"
        rest="${rest#*:}"
        [[ -x "${dir}/homebased" ]] && continue
        path="${path}${path:+:}${dir}"
    done
    echo "${path}"
}

TEST_PATH="$(isolated_path)"
readonly TEST_PATH

git_quiet() {
    git -c user.name=test -c user.email=test@example.com \
        -c init.defaultBranch=master "$@" > /dev/null
}

# commit a SKILL.md with the given body and tag it in the bare repo
publish_tag() {
    local tag="$1"
    local seed="${WORK}/seed"

    if [[ ! -d "${REMOTE}" ]]; then
        git_quiet init --bare "${REMOTE}"
        git_quiet init "${seed}"
        git_quiet -C "${seed}" remote add origin "${REMOTE}"
        mkdir -p "${seed}/.agents/skills/homebased" "${seed}/src"
        echo "outside the sparse cone" > "${seed}/src/main.rs"
    fi
    echo "skill ${tag}" > "${seed}/.agents/skills/homebased/SKILL.md"
    git_quiet -C "${seed}" add -A
    git_quiet -C "${seed}" commit -m "release ${tag}"
    git_quiet -C "${seed}" tag "${tag}"
    git_quiet -C "${seed}" push --quiet origin master "${tag}"
}

stub_homebased() {
    local version="$1"

    mkdir -p "${STUBS}"
    cat > "${STUBS}/homebased" <<STUB
#!/usr/bin/env bash
echo "homebased ${version}"
STUB
    chmod +x "${STUBS}/homebased"
}

run_sync() {
    local out="$1"

    HOMEBASED_SKILL_SRC="${CLONE}" HOMEBASED_SKILL_REPO_URL="${REMOTE}" \
        PATH="${STUBS}:${TEST_PATH}" "${SCRIPT}" > "${out}" 2>&1
}

expect_sync() {
    local tag="$1"
    local out="${WORK}/out.log"

    run_sync "${out}" || { cat "${out}" >&2; fail "sync to ${tag} failed"; }
    grep -qxF "Homebased skill at ${tag}" "${out}" \
        || fail "sync did not report ${tag}: $(cat "${out}")"
    [[ "$(cat "${SKILL_FILE}")" == "skill ${tag}" ]] \
        || fail "SKILL.md is not the ${tag} copy"
    [[ "$(git -C "${CLONE}" rev-parse HEAD)" == \
        "$(git -C "${CLONE}" rev-parse "refs/tags/${tag}^{commit}")" ]] \
        || fail "HEAD is not at ${tag}"
}

check_not_installed() {
    local out="${WORK}/absent.log"

    rm -rf -- "${STUBS}"
    run_sync "${out}" || fail "a missing homebased exited non-zero"
    grep -qxF "Skipping Homebased skill: homebased is not installed" "${out}" \
        || fail "a missing homebased was not reported"
    [[ ! -e "${CLONE}" ]] || fail "a missing homebased created the clone"
    echo "PASS: a missing homebased is skipped"
}

check_bad_version() {
    local out="${WORK}/bad.log"
    local status=0

    mkdir -p "${STUBS}"
    printf '#!/usr/bin/env bash\necho "homebased 1.2"\n' > "${STUBS}/homebased"
    chmod +x "${STUBS}/homebased"
    run_sync "${out}" || status=$?
    [[ "${status}" != 0 ]] || fail "a malformed version was accepted"
    grep -qF "homebased 1.2" "${out}" || fail "the raw version output was not shown"
    [[ ! -e "${CLONE}" ]] || fail "a malformed version created the clone"
    echo "PASS: a malformed version is refused"
}

check_first_run() {
    publish_tag v1.2.3
    # a later commit on master proves the checkout follows the tag
    publish_tag v9.9.9
    stub_homebased 1.2.3
    expect_sync v1.2.3
    [[ ! -e "${CLONE}/src/main.rs" ]] || fail "the sparse checkout included src"
    echo "PASS: the first run clones and checks out the installed tag"
}

check_repeat_offline() {
    local moved="${WORK}/remote-moved.git"

    # the recorded origin URL now points at a missing path, so any fetch fails
    mv "${REMOTE}" "${moved}"
    expect_sync v1.2.3
    mv "${moved}" "${REMOTE}"
    echo "PASS: a repeat run at the tag skips the network"
}

check_version_bump() {
    publish_tag v1.2.4
    stub_homebased 1.2.4
    # local edits in the managed clone are discarded
    echo "local edit" > "${SKILL_FILE}"
    expect_sync v1.2.4
    echo "PASS: a version bump moves the clone to the new tag"
}

check_foreign_dir() {
    local foreign="${WORK}/foreign"
    local out="${WORK}/foreign.log"
    local status
    local kind

    for kind in plain other-origin; do
        rm -rf -- "${foreign}"
        mkdir -p "${foreign}"
        echo keep > "${foreign}/keep.txt"
        if [[ "${kind}" == other-origin ]]; then
            git_quiet init "${foreign}"
            git_quiet -C "${foreign}" remote add origin https://example.com/other.git
        fi

        status=0
        HOMEBASED_SKILL_SRC="${foreign}" HOMEBASED_SKILL_REPO_URL="${REMOTE}" \
            PATH="${STUBS}:${TEST_PATH}" "${SCRIPT}" > "${out}" 2>&1 || status=$?
        [[ "${status}" != 0 ]] || fail "a ${kind} directory was accepted"
        grep -qF "${foreign}" "${out}" || fail "the refusal did not name ${foreign}"
        [[ "$(cat "${foreign}/keep.txt")" == keep ]] \
            || fail "the ${kind} directory was changed"
    done
    echo "PASS: a foreign directory is refused and left intact"
}

check_not_installed
check_bad_version
check_first_run
check_repeat_offline
check_version_bump
check_foreign_dir
