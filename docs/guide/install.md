# Installing Writfold

> **Status:** built in v0.6.0-dev.20; the installer and the manual steps install the release named below. Moved here from the README on 27 November 2026.

```sh
curl -fsSL https://writfold.dev/install.sh | sh
```

It prints its plan, downloads the static binary from the GitHub release,
checks its SHA-256, writes `~/.local/bin/writfold`, and asks before adding
the Claude Code plugin. No sudo, no edits to shell startup files. Add `-s -- --dry-run` to see the plan without installing, or
`-s -- --no-plugin` to skip the plugin.

## By hand

The same steps:

```sh
base=https://github.com/writfold-dev/writfold/releases/download/v0.6.0-dev.31
curl -LO "$base/writfold-x86_64-linux-musl" -LO "$base/writfold-x86_64-linux-musl.sha256"
echo "$(cut -d' ' -f1 writfold-x86_64-linux-musl.sha256)  writfold-x86_64-linux-musl" | sha256sum -c
install -D -m 755 writfold-x86_64-linux-musl ~/.local/bin/writfold
claude plugin marketplace add writfold-dev/writfold
claude plugin install writfold@writfold
```

The marketplace serves the plugin from `main`, which calls only hook events
the newest release has; install that release's binary (the tag above, which
a test keeps equal to the pre-release line) so the two agree.

## From source

On Linux x86_64 with [rustup](https://rustup.rs) (the pinned
toolchain installs on first build):

```sh
cargo build --release -p writfold
```


The binary is static (musl, rustls); Git is its one native dependency. Linux x86_64 or WSL2.
