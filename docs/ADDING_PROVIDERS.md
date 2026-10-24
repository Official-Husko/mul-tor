# Adding a provider

A provider is a small piece of Rust that knows how to upload one file to one host and
return the link. Most are 30–60 lines. This guide takes you from "I have a host
I want to support" to a merged provider.

## 1. Understand the host's upload protocol

Upload a small throwaway file through the host's own web page with the browser's
developer tools open (Network tab), or with `curl -v`. Write down:

* the request URL and method (`POST` multipart, raw `PUT`, chunked, several steps…)
* every form field and header that matters (the *exact* names: `files[]` and
  `file` are different to a server)
* where the link is in the reply (JSON path, a header, the redirect target…)
* the size limit, whether an account is needed, and any blocked file types

Only test against hosts and accounts you are allowed to use.

## 2. Choose the shape

| The host…                                                  | Use                                             |
|------------------------------------------------------------|-------------------------------------------------|
| runs XFileSharing (upload form `file_0`, JSON `file_code`) | add an entry to `providers/xfs.rs` — data only  |
| runs jQuery File Upload with `Content-Range` chunks        | add an entry to `providers/jqfu.rs` — data only |
| uses Dropzone-style chunks (`dzuuid`, `dzchunkindex`…)     | write a file that calls `providers/dropzone.rs` |
| anything else                                              | write a file of its own (copy the closest one)  |

Good ones to copy: `catbox.rs` (one multipart request), `pixeldrain.rs`
(authenticated raw `PUT`), `krakenfiles.rs` (find the upload server in a page
first), `ufile.rs` (session + chunks + finalise), `rapidgator.rs` (login, hash,
poll).

## 3. Write it

```rust
//! Example — multipart POST, the reply body is the link.

use async_trait::async_trait;

use crate::error::UploadError;
use crate::reply::SendReply;
use crate::provider::{Provider, ProviderInfo};
use crate::size::SizeLimit;
use crate::upload::{UploadContext, UploadedFile, new_form};

pub struct Example;

const INFO: ProviderInfo = ProviderInfo::new(
    "Example",                  // shown to users; matching ignores case and punctuation
    "https://example.com/",     // home page (used for the README table)
    "https://example.com/",     // fetched by the availability check
)
.max_size(SizeLimit::gb(2))
.api_docs("https://example.com/api");

#[async_trait]
impl Provider for Example {
    fn info(&self) -> &ProviderInfo {
        &INFO
    }

    async fn upload(&self, cx: &UploadContext<'_>) -> Result<UploadedFile, UploadError> {
        let form = new_form().part("file", cx.whole_file_part().await?);
        let reply = cx
            .http
            .post("https://example.com/upload")
            .multipart(form)
            .send_reply()
            .await?
            .ok()?;                                   // fail on anything but 2xx
        Ok(UploadedFile::new(reply.json_string("/data/url")?))
    }
}
```

`ProviderInfo` is where the *declarative* parts live, and the engine enforces them
before any request is made, so `upload` never has to:

* `.max_size(..)` — larger files fail with a clear message
* `.blocked_extensions(&[".exe"])` — lower-case, with the dot
* `.requires(&[CredentialField::new("apiKey", "API key", true)])` — the provider is
  hidden until the user configures it; read values with `cx.credential("apiKey")?`
  (use a `const` for the field list, as in `pixeldrain.rs`)
* `.known_issue("…")`, `.aliases(&[…])`, `.credentials_url(..)`

### Helpers, and the rules behind them

| Need                                     | Use                                             | Why |
|------------------------------------------|-------------------------------------------------|-----|
| a multipart form                         | `new_form()`                                    | plain `Form::new()` percent-encodes field names, turning `files[]` into something PHP-style hosts reject |
| the file as a form part                  | `cx.whole_file_part().await?` / `cx.file_part(offset, len)` | streamed from disk with progress; the form keeps a known `Content-Length` |
| the file as a raw body                   | `cx.raw_upload(cx.http.put(url)).await?`        | sets `Content-Length`, which hosts behind nginx require |
| a file name in a URL                     | `url_with_segments(base, &[&cx.file.name])?`    | escapes `?`, `#`, `/` and spaces |
| chunks                                   | `chunk_ranges(size, chunk_size)`                | |
| progress text for multi-step flows       | `cx.progress.set_message("Chunk 3/12")`         | shown next to the progress bar |
| reading the reply                        | `.ok()?`, `.expect_status(201)?`, `.json_string("/a/b")?`, `embedded_json`, `capture(&REGEX, &body)`, `csrf_meta` | errors keep a trimmed copy of the reply for `runtime.log` |
| a random id                              | `random_string(len, MIXED_ALNUM)`               | |

Rules:

1. **Make every request through `cx.http`**, never a bare `reqwest::Client`.
   That is what applies the user's proxy and user agent, and what lets tests
   redirect the host to a local server.
2. **Never put a total timeout on a request that carries file data.** In reqwest
   the timeout also covers waiting for the response, so it would abort any
   upload slower than the timeout. Small API calls are fine; `cx.http.get` already
   has one.
3. **Return errors, do not log them.** The engine logs failures with the provider name,
   the file and the trimmed reply. Use `UploadError::protocol(msg, &reply.body)`
   so that excerpt is attached.
4. **Never print or log credentials**, and do not put them in URLs you build
   without good reason (error messages strip the query string, but be careful).
5. If a value from the reply ends up in a URL you request (a server name, say),
   check it is what you expect (see `gofile.rs`, `onefichier.rs`).

## 4. Register it

In `crates/core/src/providers/mod.rs`: add `mod example;`, `pub use example::Example;`
and `Arc::new(Example)` to `all()`. (Entries in `xfs.rs` / `jqfu.rs` need nothing
else.) The list is sorted by name automatically.

## 5. Test it

Tests run the provider against an in-process server that records every request, so
nothing touches the real host. See `crates/core/tests/common/mod.rs` and the
existing tests in `providers_simple.rs` / `providers_flows.rs`:

```rust
#[tokio::test]
async fn example_posts_the_file_and_reads_the_link() {
    let mock = Mock::start(|r| {
        if r.is("POST", "example.com", "/upload") {
            Reply::json(serde_json::json!({"data": {"url": "https://example.com/f/1"}}))
        } else {
            Reply::not_found()
        }
    })
    .await;

    let run = run_default(&*builtin("example"), &mock).await;

    assert_eq!(run.url(), "https://example.com/f/1");
    let request = &mock.requests()[0];
    assert_eq!(field(&request.multipart(), "file").data, b"0123456789abcdef");
    request.assert_not_chunked();
}
```

Test what a server would actually see: field names, headers, the order and size
of chunks, and what happens when the host answers with an error. Chunked providers
should expose `with_chunk_size` (see `ufile.rs`) so a test can use a 10-byte
chunk instead of a real one.

```sh
cargo test -p mul-tor-core                                  # the provider tests
cargo test --workspace --features mul-tor/test-hooks        # everything
cargo clippy --workspace --all-targets --features mul-tor/test-hooks
```

## 6. Update the README

```sh
cargo run -q -p mul-tor -- providers --format markdown
```

prints the table used in the README. A test fails if a registered provider is missing
from it, so you cannot forget.

## 7. Try it for real (optional, but worth it)

Once the tests pass, upload a small throwaway file from a scratch folder:

```sh
mul-tor upload hello.txt --provider example --home /tmp/mul-tor-try -vv
```

Mock-server tests prove you send what you meant to send; only the real host can
tell you that is what it wants. Say in the pull request whether you did this.
