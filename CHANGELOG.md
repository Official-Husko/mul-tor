# Changelog

## 2.0.0 (unreleased)

Mul-Tor is rewritten in Rust.

### Added
* `mul-tor`, a single native command-line program for Linux, macOS and Windows
  (no Python, no installer), with a scriptable `upload` command (`--json`, exit
  codes), `providers`, `presets`, `check` and `config` commands.
* Logging that says when, where and for what: local timestamps with milliseconds
  and UTC offset, level, module and source line, the upload (provider, file) a
  line belongs to, how long it took, and every request with its query string
  removed. Daily files (last 14 kept) in a `logs` folder, optional JSON lines,
  `MUL_TOR_LOG` to change the detail, `-v`/`-vv`/`-vvv` on the terminal and
  `mul-tor logs` to read them.
* A recorded state for every provider (unverified, working, degraded, broken),
  shown in `mul-tor providers` and the window; broken ones are skipped by
  `--all` and presets.
* A full-screen terminal interface (`mul-tor` with no arguments): a folder
  browser, a provider checklist with filter, status and presets, a summary of what
  will be sent before it starts, live progress with speed and time left, a log pane
  and copy-all-links. Pasting or dropping paths works anywhere on the first step.
* `mul-tor-gui`, a graphical version (egui on wgpu) built on the same engine,
  with a log window and a status column for every provider.
* Several uploads at once (`--parallel`), per-upload progress bars (including
  chunked and multi-step uploads), and clean cancellation with Ctrl-C.
* Your own proxy (`http://`, `socks5://`, `socks5h://`, so Tor works).
* Settings in a commented `config.toml` in the platform's config folder (or one
  folder with `--home`); an existing `config.json` is imported on first run.
* Credentials for providers that need an account; those providers are hidden until set.
* `docs/ADDING_PROVIDERS.md` and a test harness that checks each provider's exact HTTP
  requests against a local mock server.
* Providers that take anonymous uploads, no account needed: 0x0.st, AdminForge, Ayaya.beauty,
  curl.by, Datanodes, Disroot, Dosya.co, DropMeFiles, EasySend, End2End.tech, FileMirage,
  Filespace, GigaFile, Qu.ax, Send.adminforge.de, Send.cyberjake.xyz, Send.mni.li,
  Send.monks.tools, Send.now, Sendspace, SendVis, Storage.to, Temp.sh, TempFile.org, tfLink,
  Uguu, VikingFile and X0.at. All but four were uploaded to for real and are marked
  `working`. 0x0.st (not reachable from the machine they were tried on), Datanodes (its
  download page answered 404 to a plain client), Send.now (its file pages sit behind a
  browser check) and DropMeFiles (a small file went up and came back intact, but its
  throttle stopped the test of a file in several chunks) stay `unverified`. Where a host offers a choice of retention, the longest is always asked for.
* Lufi instances (AdminForge, Disroot): the file is encrypted here with a random key, slice
  by slice, and sent over a WebSocket, as the instance's web page does, so its own download
  page opens it; the key is only in the part of the link after `#`.
* Send instances (SendVis and four others): the Firefox Send fork's protocol, as its web page
  speaks it. The file is encrypted here (RFC 8188 `aes128gcm`, a random 128-bit secret),
  sent over a WebSocket, and the secret stays in the part of the link after `#`. The
  longest expiry and most downloads each instance's own `/config` allows are always asked for.
* Sendspace (a signed upload form read off its home page) and DropMeFiles (a transfer made
  over its page's own API, the file sent in raw chunks with the 14 days it offers at most;
  it answers "Spam" to a second transfer from the same address within a few minutes).
* The XFileSharing family reads two more ways of finishing an upload (older versions
  redirect to a result page) and takes the upload node from the page's own attribute.
* A provider can list the only file types it takes (Qu.ax), next to the list it refuses.

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
