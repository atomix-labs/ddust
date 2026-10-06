# The repository's own recipes go here, above the block the just profile writes.

# Holds the bench workspace's lints, a copy of the root's `[workspace.lints]`, equal to them.
check-bench-lints:
    #!/usr/bin/env -S mise exec -- python3
    import sys, tomllib
    root = tomllib.load(open("Cargo.toml", "rb"))["workspace"]["lints"]
    bench = tomllib.load(open("crates/ddust/bench/Cargo.toml", "rb"))["lints"]
    if root != bench:
        sys.exit("crates/ddust/bench/Cargo.toml: its [lints] differ from the root's [workspace.lints]; copy them again")
    print("bench lints: the root's")

# Runs the benches twice, on an isolated CPU on Linux, into `crates/ddust/bench/results/` with a
# manifest: `just bench-run 3 baseline graviton4`, and `--cpu`, `--passes`, `--filter` or
# `--purpose` after. Each word reaches the script as one argument, quoted.
[positional-arguments]
bench-run pr subject host *options:
    cd crates/ddust/bench && pr="$1" subject="$2" host="$3" && shift 3 && mise exec -- python3 scripts/run.py --pr "$pr" --subject "$subject" --host "$host" "$@"

# Builds the bench's probe, prints each kernel's instructions with those of every function it calls,
# and holds ddust's kernels to dividing by constants alone. The recipes export `CARGO_BUILD_TARGET`,
# so cargo builds the probe under the target's own directory.
bench-assembly:
    cd crates/ddust/bench && mise exec -- cargo build --locked --profile bench --example probe
    cd crates/ddust/bench && mise exec -- python3 scripts/assembly.py sizes "target/$CARGO_BUILD_TARGET/release/examples/probe"
    cd crates/ddust/bench && mise exec -- python3 scripts/assembly.py gate "target/$CARGO_BUILD_TARGET/release/examples/probe"

# >>> devset: just >>>
# Each active profile's recipes.
import? '.just/agents.just'
import? '.just/cargo-bump.just'
import? '.just/cargo-deny.just'
import? '.just/cargo-hack.just'
import? '.just/cargo-manifest.just'
import? '.just/cargo-nextest.just'
import? '.just/cargo-profiles.just'
import? '.just/cargo-publish.just'
import? '.just/cargo-unused.just'
import? '.just/cargo-workspace.just'
import? '.just/devset.just'
import? '.just/dprint.just'
import? '.just/editorconfig.just'
import? '.just/git-attributes.just'
import? '.just/git-changelog.just'
import? '.just/git-commits.just'
import? '.just/git-ignore.just'
import? '.just/github-automation.just'
import? '.just/github-bump.just'
import? '.just/github-ci.just'
import? '.just/github-dependabot.just'
import? '.just/github-labels.just'
import? '.just/github-nightly.just'
import? '.just/github-release.just'
import? '.just/github-templates.just'
import? '.just/github-watch.just'
import? '.just/github-workflow-lint.just'
import? '.just/just.just'
import? '.just/lychee.just'
import? '.just/markdown.just'
import? '.just/mdbook.just'
import? '.just/mdbook-tool.just'
import? '.just/mise.just'
import? '.just/project.just'
import? '.just/rust.just'
import? '.just/rust-clippy.just'
import? '.just/rust-doc.just'
import? '.just/rust-fmt.just'
import? '.just/rust-lints.just'
import? '.just/rust-toolchain.just'
import? '.just/setup.just'
import? '.just/shell.just'
import? '.just/spelling.just'
import? '.just/toml.just'
import? '.just/vscode.just'
import? '.just/yaml.just'

# Runs every `check-*` recipe, as CI does, and names each that fails.
check: (_each "check")

# Runs every `fix-*` recipe.
fix: (_each "fix")

# Runs every `bump-*` recipe: each moves what its profile pins, and reports to $BUMP_REPORT_DIR.
bump: (_each "bump")

# Runs every `nightly-*` recipe: the checks too slow for every change.
nightly: (_each "nightly")

# Runs every `test-*` recipe: the suites too slow for `just check`, which CI runs beside it.
test: (_each "test")

# Runs every `setup-*` recipe: what a checkout needs before it builds. `mise bootstrap` runs it.
setup: (_each "setup")

# Runs every `host-*` recipe: the machine's own setup, whose steps may ask for sudo.
host: (_each "host")

# Runs every `release-*` recipe for $RELEASE_VERSION: each writes what a release needs.
release: (_each "release")

# Runs every `package-*` recipe: what a release ships, built for this machine into dist/.
package: (_each "package")

# Runs every `publish-*` recipe: what a release puts in a registry, once the release is out.
publish: (_each "publish")

# Runs every recipe named `<verb>-*`, and names each that fails.
_each verb:
    #!/usr/bin/env bash
    set -uo pipefail
    failed=()
    for recipe in $(just --justfile '{{ justfile() }}' --summary); do
        [[ $recipe == {{ verb }}-* ]] || continue
        just --justfile '{{ justfile() }}' "$recipe" || failed+=("$recipe")
    done
    if (( ${#failed[@]} )); then
        echo "failed: ${failed[*]}" >&2
        exit 1
    fi

# <<< devset: just <<<
