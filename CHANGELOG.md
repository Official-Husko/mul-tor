# Changelog

## 2.0.0 (unreleased)

Mul-Tor is rewritten in Rust.

### Added
* `mul-tor`, a single native command-line program for Linux, macOS and Windows
  (no Python, no installer), with a scriptable `upload` command (`--json`, exit
  codes), `providers`, `presets`, `check` and `config` commands.
* `mul-tor-gui`, a graphical version (egui on wgpu) built on the same engine.
* Several uploads at once (`--parallel`), per-upload progress bars (including
  chunked and multi-step uploads), and clean cancellation with Ctrl-C.
* Your own proxy (`http://`, `socks5://`, `socks5h://`, so Tor works).
* Settings in a commented `config.toml` in the platform's config folder (or one
  folder with `--home`); an existing `config.json` is imported on first run.
* Credentials for providers that need an account; those providers are hidden until set.
* `docs/ADDING_PROVIDERS.md` and a test harness that checks each provider's exact HTTP
  requests against a local mock server.

### Changed
* All 37 providers were ported. Providers sharing a protocol now share code (XFileSharing,
  jQuery File Upload, Dropzone).
* Provider availability is checked concurrently, and a provider that answers 401/403/429
  counts as up (it still accepts uploads).
* Update checks compare versions as versions (the old string comparison would
  have broken at 1.10) and only notify; nothing is downloaded or installed.
* Large files are streamed from disk and sent with a known length, never loaded
  into memory.
* Long file names are shortened without losing their extension.
* Empty files are refused up front with a clear message (the old code sent them
  and left the outcome to the host).
* Size limits, blocked file types and missing credentials are checked before any
  request is made, so a doomed upload no longer costs a round-trip.

### Fixed
* `Transfer.sh` and `Download.gg` could never be selected (their names differed
  between two tables); name matching now ignores case and punctuation.
* Proxies were configured for plain `http://` only, so they were silently
  ignored for every HTTPS provider; they apply to all traffic now.
* An outdated `user_agents.json` made the old code return no user agents at all.
* Gofile retried forever when its API was down; it now gives up after 5 tries.
* Fastupload replaced its session cookie with the text `False` when a chunk
  reply did not set one.
* `Transfer.sh` uploads wrapped the file in a multipart body that the host
  stores verbatim; the file is now sent as the plain body.
* Passwords sent in query strings (Rapidgator) can no longer leak into error text.
* `saveLinksToFile` is honoured (it was read but never used).

### Removed
* The Python implementation. It remains in the git history (tag `1.5.2`).
* Self-replacing updates and the Windows-only build.
