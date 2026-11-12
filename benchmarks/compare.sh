#!/usr/bin/env bash
# Counts the instructions and allocations of annotated's cases at a commit, then in the working tree, and lists
# every case that runs more than 1% more instructions, or allocates more blocks or bytes. Exits with status 3
# when there is one, and 0 when there is none.
#
#   benchmarks/compare.sh [commit]    # HEAD by default
#
# Needs Valgrind, and gungraun-runner at the version of gungraun in benchmarks/Cargo.lock.
set -euo pipefail

before=${1:-HEAD}
benchmarks=$(cd "$(dirname "$0")" && pwd)
export CARGO_TARGET_DIR=${CARGO_TARGET_DIR:-$benchmarks/target}

# Both sides are built and run from this one checkout: a process started from another directory lays its
# memory out a little differently, and the same code then runs up to 0.5% more or fewer instructions.
checkout=$CARGO_TARGET_DIR/compare

# A commit of every file in the working tree, tracked or not, made through an index of its own so the
# working tree and the real index are left as they are.
snapshot_of_working_tree() {
    local index
    index=$(mktemp)
    cp "$(git -C "$benchmarks" rev-parse --path-format=absolute --git-path index)" "$index"
    GIT_INDEX_FILE=$index git -C "$benchmarks/.." add --all
    local tree
    tree=$(GIT_INDEX_FILE=$index git -C "$benchmarks" write-tree)
    rm "$index"
    git -C "$benchmarks" -c user.name=compare.sh -c user.email=compare.sh commit-tree "$tree" -p HEAD \
        -m "the working tree, to compare"
}

count_instructions_at() {
    local commit=$1
    shift
    git -C "$checkout" checkout --quiet --detach "$commit"
    (cd "$checkout/benchmarks" && cargo bench --bench instructions -- "$@")
}

working_tree=$(snapshot_of_working_tree)
git -C "$benchmarks" worktree remove --force "$checkout" 2>/dev/null || true
trap 'git -C "$benchmarks" worktree remove --force "$checkout"' EXIT
git -C "$benchmarks" worktree add --quiet --detach "$checkout" "$before"

if [[ ! -f $checkout/benchmarks/benches/instructions.rs ]]; then
    echo "$before has no instructions benchmark to compare with, so this counts the working tree alone." >&2
    count_instructions_at "$working_tree"
    exit
fi

count_instructions_at "$before" --save-baseline=before
count_instructions_at "$working_tree" --baseline=before \
    --callgrind-limits='ir=1%' --dhat-limits='totalblocks=0%,totalbytes=0%'
