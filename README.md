# git-village

Draw one developer's public commits as a looping pixel-art city for their own GitHub profile README. Only public commits count, whichever token is used. Issues, reviews, pull requests, private commits, and calendar contribution totals do not count.

```html
<img src="village/village.gif" alt="My commit village">
```

Set up the nightly workflow in [Getting started](docs/getting-started.md). The composite Action requires published release binaries; until those exist, use the included workflow that installs from source.

```sh
cargo install --path .
git-village collect octocat --output snapshot.json
git-village render snapshot.json --output-dir village
```

Collection requires an authenticated [GitHub CLI](https://cli.github.com/). Rendering uses the snapshot alone, without network access or a clock. Outputs are `village.gif`, `village.png`, and `preview.html`, with transparent PNG and GIF exports cropped to the loop’s shared bounds plus a four-pixel native margin; scene exports stay 768 × 512 pixels. The Earth setting is the only setting in v1.

Lifetime public commits control growth; yearly commits control population; monthly commits control traffic; weekly commits control construction. Each primary language earns a permanent peninsula at 25 lifetime commits; its own lifetime count grows its land and buildings. Languages without arms share the hub. The status strip shows login, stage, commits, population, mood, and language swatches. See [the design](docs/design.md) for thresholds and mood rules. At the GitHub repository cap the collector splits windows, recording `detail_truncated` if a single day still hits the cap. Snapshots store no repository names.

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --release --example generate-sheet -- target/review
```

The review generator writes two contact sheets and six stage GIFs. The deterministic library exposes collection, snapshot validation, development, mood, setting materials, layout, depth rasterization, motion, rendering, and encoding as separate modules.

Inspired by [git-koi](https://github.com/yalongwastaken/git-koi), by Anthony Yalong. Adapted conventions and code are credited in [NOTICE](NOTICE). MIT license, copyright Rei Tamaru.

The village is drawn on a transparent canvas, so it sits directly on GitHub's light or dark page. Add `--background scene` for the sky, sea, and status strip.
