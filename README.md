# 🏕️ Campfire

A tiny static site generator, greatly inspired by Zola.

## Why?

Inspired by Tom Critchlow's article on [Building a Digital Garden](https://tomcritchlow.com/2019/02/17/building-digital-garden/), there's three distinct concepts of information flows:
- Streams
- Campfires
- Gardens

While my personal notes are, for a lack of public access, not quite a digital garden, I very much would benefit from bringing my blog and my notes closer together.

The result is 🏕 Campfire: a tool to stitch my collection of notes into more worthwhile, more curated stories to share.

With a few minor, Markdown-related inconveniences where GitHub Flavored Markdown doesn't support footnotes, this also allows me to view (and presumably edit) my notes on GitHub.

## Writing
I'm writing my notes in [Obsidian](https://obsidian.md/), which is for the most part somewhat reasonably formatted markdown, if you disable wiki-like links.

That said, 🏕 Campfire is reasonably tool-independent, although certain choices have been made with my personal tools in mind:

- The configuration as well as created files are stored in the `.campfire`, which is invisible within Obsidian.
- Inline footnotes with `^[my footnote]` are reasonably well-supported and are perhaps the biggest deviation from standard markdown that I'm currently actively using.
- The output is rather minimally formatted, and a work-in-progress.

## Building your Site
For the `example-vault` included, building should be as straightforward as:

```shell
cargo install campfire
campfire -b example-vault build
```

# Caveats

- Error handling is very spotty
- Far from idiomatic Rust code, it's more of a hands-on exercise to building your own static site generator for me
