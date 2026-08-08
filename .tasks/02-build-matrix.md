# 02. Build matrix

## What to build

A release workflow that turns a `v*` tag into four runnable binaries, each with
the viewer inside it. This task stops short of publishing anything — the legs
upload their binaries as workflow artifacts, which is enough to prove every
target builds and runs before a Release is put in front of anyone.

The viewer is built **once**, in its own job, and passed to the build legs as an
artifact. `rust-embed` reads `web/dist` at compile time, so each leg needs the
built viewer on disk before `cargo build` — but the vite output is
platform-independent, so building it four times would cost four times over and
invite the four legs to disagree about what they embedded.

Each leg builds on a runner of its own architecture, so nothing is
cross-compiled and every leg can run the binary it just produced:

| Asset | Runner | Target |
| --- | --- | --- |
| `askance-linux-x64` | `ubuntu-24.04` | `x86_64-unknown-linux-musl` |
| `askance-linux-arm64` | `ubuntu-24.04-arm` | `aarch64-unknown-linux-musl` |
| `askance-macos-x64` | `macos-15-intel` | `x86_64-apple-darwin` |
| `askance-macos-arm64` | `macos-15` | `aarch64-apple-darwin` |

Two things the Linux legs need: `musl-tools`, because `libsqlite3-sys` bundles
SQLite's C source and `ring` compiles C of its own, and the musl target added to
the pinned toolchain. Build the binary's package by name rather than the whole
workspace — a workspace-wide build unifies `askance-render`'s TypeScript
emitter, which is a test's business, into the release binary. `nix/askance.nix`
makes the same choice for the same reason.

Pin everything, as the surrounding workflows do: actions to released versions,
runners to images rather than `-latest`, and Rust to the patch version the
flake's nixpkgs carries.

## Acceptance criteria

- [ ] A pushed `v*` tag runs the workflow; the viewer is built in one job and
      consumed by all four build legs
- [ ] All four legs succeed and upload their binary as an artifact
- [ ] Each leg runs the binary it just built: `askance --help` works, and
      `askance serve` answers an HTTP request with the embedded viewer rather
      than the absent-viewer fallback
- [ ] The two Linux binaries are statically linked — `ldd` reports them as not
      dynamic executables
- [ ] The release binary does not carry the TypeScript emitter
