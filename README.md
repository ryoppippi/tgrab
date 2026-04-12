# agent-fetcher

A CLI tool that fetches text content from services that normally require a browser or login — YouTube transcripts, Twitter/X posts, and Bluesky posts.

## Usage

```sh
# Run directly without installing
nix run github:ryoppippi/agent-fetcher -- <url>
```

The service is detected automatically from the URL.

### YouTube — transcript

```sh
nix run github:ryoppippi/agent-fetcher -- https://www.youtube.com/watch?v=dQw4w9WgXcQ
nix run github:ryoppippi/agent-fetcher -- https://youtu.be/dQw4w9WgXcQ

# Specify language
nix run github:ryoppippi/agent-fetcher -- --lang ja https://www.youtube.com/watch?v=dQw4w9WgXcQ
```

### Twitter / X — post text

```sh
nix run github:ryoppippi/agent-fetcher -- https://x.com/user/status/1234567890
nix run github:ryoppippi/agent-fetcher -- https://twitter.com/user/status/1234567890
```

### Bluesky — post text

```sh
nix run github:ryoppippi/agent-fetcher -- https://bsky.app/profile/user.bsky.social/post/abc123
```

> For more complex pages requiring JavaScript rendering or authentication, use [agent-browser](https://github.com/vercel-labs/agent-browser) instead.

## Supported URL patterns

| Service | Patterns |
|---|---|
| YouTube | `youtube.com/watch?v=`, `youtu.be/`, `youtube.com/embed/`, `youtube.com/v/`, `m.youtube.com/watch?v=` |
| Twitter / X | `x.com/*/status/*`, `twitter.com/*/status/*`, `www.twitter.com/*/status/*` |
| Bluesky | `bsky.app/profile/*/post/*` |

## Agent Skill

This repo ships a skill compatible with the [Agent Skills Specification](https://agentskills.io).

### Install via skills CLI

```sh
npx skills add ryoppippi/agent-fetcher
```

<details>
<summary>Install via Nix (agent-skills-nix)</summary>

Using [agent-skills-nix](https://github.com/Kyure-A/agent-skills-nix) with Home Manager:

**flake.nix**
```nix
inputs = {
  agent-skills.url = "github:Kyure-A/agent-skills-nix";
  agent-fetcher = {
    url = "github:ryoppippi/agent-fetcher";
    flake = false;
  };
};
```

**home.nix**
```nix
programs.agent-skills = {
  enable = true;
  sources.ryoppippi = {
    input = "agent-fetcher";
    subdir = "skills";
    idPrefix = "ryoppippi";
  };
  skills.enable = [ "ryoppippi/agent-fetcher" ];
  targets.claude.enable = true;
};
```

</details>

## Development

Requires [Nix](https://nixos.org/) with flakes enabled.

```sh
# Enter dev shell (provides cargo, rustc, clippy, rust-analyzer, treefmt)
nix develop

# Run tests
cargo test

# Run integration tests (requires network)
cargo test -- --ignored

# Format
nix fmt

# Build binary
nix build
./result/bin/agent-fetcher --help

# Run all checks (tests + format)
nix flake check
```

## Acknowledgements

- [fetch-mcp](https://www.npmjs.com/package/fetch-mcp) — original inspiration for this project
- [impit](https://github.com/apify/impit) — browser-impersonating HTTP client used for YouTube requests
- [FxTwitter](https://github.com/FixTweet/FxTwitter) — Twitter/X embed proxy providing the JSON API
- [Bluesky AT Protocol](https://atproto.com/) — public API used to fetch Bluesky posts
- [@playzone/youtube-transcript](https://www.npmjs.com/package/@playzone/youtube-transcript) — reference implementation for the Innertube API approach
- [crane](https://github.com/ipetkov/crane) — Nix library for building Rust packages with git dependencies

## License

[MIT](./LICENSE)
