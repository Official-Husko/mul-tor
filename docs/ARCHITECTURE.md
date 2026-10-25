# How Mul-Tor is built

Three crates in one Cargo workspace. The two programs are thin: anything both need
lives in the core.

| Crate | Produces | What it is |
|---|---|---|
| `crates/core` | `mul-tor-core` (library) | providers, the upload engine, settings, presets, logging; no user interface |
| `crates/cli` | `mul-tor` | `mul-tor upload FILES -p PROVIDER` for scripts, or `mul-tor` alone for the full-screen interface |
| `crates/gui` | `mul-tor-gui` | an egui/wgpu window over the same engine |

`cargo build` builds the core and the command-line program; the window pulls in the
whole egui/wgpu stack and is built with `cargo build -p mul-tor-gui`.

## The core (`crates/core/src`)

| Module | Holds |
|---|---|
| `provider/` | the `Provider` trait, `ProviderInfo` (limits, credentials, recorded reliability), the offline pre-checks, the registry |
| `providers/` | one file per provider, plus the shared protocols (`form_post`, `xfs`, `lufi`, `send`, `jqfu`, `dropzone`), and `websocket` for the hosts that upload over one |
| `upload/` | the local file, the request context (streams file data with progress), chunking, URL and random helpers |
| `engine/` | plans the jobs, runs them with bounded parallelism, isolates crashes, reports `Event`s |
| `net/` | the HTTP client factory, reply parsing, proxy and user-agent lists, reachability probes, update checks |
| `storage/` | `config.toml`, presets, the link files, the platform directories |
| `logging/` | timestamps and structure, daily files, the in-memory buffer the interfaces show, secret redaction |
| `setup.rs`, `files.rs`, `size.rs`, `error.rs`, `progress.rs` | glue and small shared types |

The engine knows nothing about terminals or windows. It takes files and providers and
calls back with events (`Queued`, `Started`, `Progress`, `Finished`); both programs
turn those into what the user sees.

## The terminal interface (`crates/cli/src/tui`)

A model/update/view split, so almost all of it is tested without a terminal:

- `model/`: what is on screen. `Model::update(Msg) -> Vec<Cmd>` changes it and says what
  to do next. It never touches a terminal, a thread or the network.
- `view/`: draws the model with ratatui. Tested against a fake terminal at many sizes
  and in all three colour modes.
- `runtime.rs`: carries out the commands (uploads, probes, the clipboard) and answers
  with messages. `events.rs` feeds in keys, pasted paths and a clock; `terminal.rs`
  owns the screen and always gives it back.

The window (`crates/gui/src`) follows the same idea: `model/` is plain data, `app/`
handles what the user asked for, `view/` only draws, `tasks/` runs things in the
background and reports back on a channel.

## Rules the code follows

- **No file over 250 lines**, tests included (`scripts/check-file-sizes.sh`, run in CI).
  Split by what a file is for; a `mod.rs` names its submodules and holds at most the
  module's central type, not the code that works on it.
- **Security.** Credentials, tokens, cookies and query strings are never logged or put
  in error messages: `Credentials` prints only its key names, URLs are logged through
  `logging::redact_url`, and network errors drop their query string. The config file is
  readable only by its owner. Anything a server sends is checked before it becomes part
  of a URL or path. `unsafe` is forbidden.
- **Performance.** File data is streamed, never read whole into memory; requests that
  carry file data have no total timeout; hashing runs off the async threads.
- **Logging** uses `tracing` with fields and spans (`run`, `job`), never `println!`.
  Files are rotated daily; `MUL_TOR_LOG` changes the level.
- **Naming** is standard Rust: `PascalCase` types, `snake_case` everything else,
  `SCREAMING_SNAKE_CASE` constants. Keys in the config file keep their documented
  spelling (`apiKey`).
- **Versions** are `MAJOR.MINOR.PATCH`, shared by all crates in
  `[workspace.package]`: a breaking change bumps the major, a new feature the minor, a
  fix the patch. Update `CHANGELOG.md` in the same commit that bumps it.
- **Commits** follow Conventional Commits (`feat`, `fix`, `refactor`, `docs`, `test`,
  `chore`, `ci`, `perf`), one logical change each, and every commit builds and passes
  the tests.
- **Providers are honest about their state.** Each one is `Unverified` until someone has
  seen it work against the real service, and `Degraded` or `Broken` (with a reason) when
  that is known. Tests never talk to a real host: they use an in-process mock server.

## Testing

```sh
cargo test --workspace --features mul-tor/test-hooks
cargo clippy --workspace --all-targets --features mul-tor/test-hooks -- -D warnings
cargo fmt --all --check
scripts/check-file-sizes.sh
```

- `crates/core/tests/common/` is a mock HTTP server that records every request; the
  provider tests check the exact fields, headers and chunk boundaries each one sends.
- `crates/cli/tests/e2e/` runs the real binary against that kind of server.
- The window is tested headlessly with egui's `kittest`, which also fails on text the
  bundled fonts cannot draw and on controls without an accessible name.
