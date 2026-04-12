# agent-fetcher

A CLI tool that fetches text content from services that normally require a browser or login — YouTube transcripts, Twitter/X posts, and Bluesky posts.

## Usage

```sh
agent-fetcher <url>
```

The service is detected automatically from the URL.

### YouTube — transcript

```sh
agent-fetcher https://www.youtube.com/watch?v=dQw4w9WgXcQ
agent-fetcher https://youtu.be/dQw4w9WgXcQ

# Specify language
agent-fetcher --lang ja https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### Twitter / X — post text

```sh
agent-fetcher https://x.com/user/status/1234567890
agent-fetcher https://twitter.com/user/status/1234567890
```

Uses [FxEmbed](https://github.com/FxEmbed/FxEmbed) (`fixupx.com` / `fxtwitter.com`) as a proxy.

### Bluesky — post text

```sh
agent-fetcher https://bsky.app/profile/user.bsky.social/post/abc123
```

Uses [FxEmbed](https://github.com/FxEmbed/FxEmbed) (`fxbsky.app`) as a proxy.

## Supported URL patterns

| Service | Patterns |
|---|---|
| YouTube | `youtube.com/watch?v=`, `youtu.be/`, `youtube.com/embed/`, `youtube.com/v/`, `m.youtube.com/watch?v=` |
| Twitter / X | `x.com/*/status/*`, `twitter.com/*/status/*`, `www.twitter.com/*/status/*` |
| Bluesky | `bsky.app/profile/*/post/*` |

## Development

Requires [Nix](https://nixos.org/) with flakes enabled.

```sh
# Enter dev shell (provides cargo, rustc, clippy, rust-analyzer, treefmt)
nix develop

# Run tests
cargo test

# Format
nix fmt

# Build binary
nix build
./result/bin/agent-fetcher --help

# Run all checks (tests + format)
nix flake check
```

## Implementation notes

- HTTP client: [impit](https://github.com/apify/impit) with Firefox 144 TLS fingerprinting, bypassing bot detection on YouTube
- YouTube transcripts: parsed from the `playerCaptionsTracklistRenderer` JSON embedded in the page, then fetched as TimedText XML
- Twitter/X and Bluesky: URL-rewritten to FxEmbed proxies; post text extracted from `og:description`
- Nix build: [crane](https://github.com/ipetkov/crane) handles git dependencies and `[patch.crates-io]` via `cargo vendor`
