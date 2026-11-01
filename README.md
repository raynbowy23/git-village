# git-village

Draw one developer's public commits as a looping pixel-art city for their own GitHub profile README. Only public commits count, whichever token is used. Issues, reviews, pull requests, private commits, and calendar contribution totals do not count.

<p align="center">
  <img src="docs/village.gif" alt="The author's village, drawn from 1,697 public commits, with four language peninsulas around a central hub">
</p>

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

The stills below come from a sample account and use `--background scene`, with the status strip cropped out.

A village grows through six stages as lifetime commits add up.

<table>
  <tr>
    <td align="center"><img src="docs/gallery/outpost.png" alt="Outpost: a small grey island with a tent"><br>Outpost, 0 commits</td>
    <td align="center"><img src="docs/gallery/hamlet.png" alt="Hamlet: a small green island with a few buildings"><br>Hamlet, 25 commits</td>
    <td align="center"><img src="docs/gallery/village.png" alt="Village: an island with its first short road arms"><br>Village, 250 commits</td>
  </tr>
  <tr>
    <td align="center"><img src="docs/gallery/town.png" alt="Town: longer peninsulas lined with houses"><br>Town, 1,000 commits</td>
    <td align="center"><img src="docs/gallery/city.png" alt="City: full peninsulas with a block of towers"><br>City, 5,000 commits</td>
    <td align="center"><img src="docs/gallery/metropolis.png" alt="Metropolis: peninsulas with much taller towers"><br>Metropolis, 20,000 commits</td>
  </tr>
</table>

The most recent commits set the mood of each day's loop. A fifth mood, Workday, is any other day.

<table>
  <tr>
    <td align="center"><img src="docs/gallery/market-day.png" alt="Market day: a sunny city with stalls in the plaza"><br>Market day, the first commit day of the week</td>
    <td align="center"><img src="docs/gallery/festival.png" alt="Festival: a sunny city with fireworks in the sky"><br>Festival, after a milestone or a busy week</td>
  </tr>
  <tr>
    <td align="center"><img src="docs/gallery/night.png" alt="Night: a dark city under a moon with lit windows"><br>Night, after 14 quiet days</td>
    <td align="center"><img src="docs/gallery/abandoned.png" alt="Abandoned: a grey, overcast city with no traffic"><br>Abandoned, after 60 quiet days</td>
  </tr>
</table>

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --release --example generate-sheet -- target/review
```

The review generator writes two contact sheets and six stage GIFs. The deterministic library exposes collection, snapshot validation, development, mood, setting materials, layout, depth rasterization, motion, rendering, and encoding as separate modules.

Inspired by [git-koi](https://github.com/yalongwastaken/git-koi), by Anthony Yalong. Adapted conventions and code are credited in [NOTICE](NOTICE). MIT license, copyright Rei Tamaru.

The village is drawn on a transparent canvas, so it sits directly on GitHub's light or dark page. Add `--background scene` for the sky, sea, and status strip.
