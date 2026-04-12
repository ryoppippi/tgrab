---
name: agent-fetcher
description: Fetch text content from YouTube transcripts, Twitter/X posts, and Bluesky posts. Use this when the user shares a YouTube, Twitter/X, or Bluesky URL and wants to read its content.
---

# agent-fetcher

## When to Use This Skill

Use this skill when the user provides a URL from one of these services and wants to read or summarise its content:

- A YouTube video URL (to fetch the transcript)
- A Twitter / X post URL
- A Bluesky post URL

For more complex pages requiring JavaScript rendering or authentication, use the [agent-browser](https://github.com/vercel-labs/agent-browser) skill instead.

## How to Fetch Content

Always run via a subagent to keep the main conversation context clean.

```sh
nix run github:ryoppippi/agent-fetcher -- <url>
```

### Examples

```sh
# YouTube transcript
nix run github:ryoppippi/agent-fetcher -- https://www.youtube.com/watch?v=dQw4w9WgXcQ

# YouTube transcript in Japanese
nix run github:ryoppippi/agent-fetcher -- --lang ja https://www.youtube.com/watch?v=dQw4w9WgXcQ

# Twitter / X post
nix run github:ryoppippi/agent-fetcher -- https://x.com/user/status/1234567890

# Bluesky post
nix run github:ryoppippi/agent-fetcher -- https://bsky.app/profile/user.bsky.social/post/abc123
```

## Supported URL Patterns

| Service     | Patterns                                                                                   |
| ----------- | ------------------------------------------------------------------------------------------ |
| YouTube     | `youtube.com/watch?v=`, `youtu.be/`, `youtube.com/embed/`, `m.youtube.com/watch?v=`       |
| Twitter / X | `x.com/*/status/*`, `twitter.com/*/status/*`                                               |
| Bluesky     | `bsky.app/profile/*/post/*`                                                                |

## Output Format

- **YouTube**: Title as `# heading`, then each caption line with timestamp `[M:SS] text`
- **Twitter / X**: `@username:\n\ntweet text`
- **Bluesky**: `@handle on Bluesky:\n\npost text`
