# git-village

Git commits build a tiny pixel-art settlement that loops on a profile README. Code languages paint the rooftops, all-time commit counts expand the city limits, streaks set the town atmosphere, and the most recent push decides the day's activity. A nightly workflow keeps the animation updated, the binary runs straight from the command line, and the Rust engine is available on crates.io.

<p align="center">
  <img src="docs/village.gif" alt="The author's village, drawn from 1,697 public commits: a round island of mixed city blocks around a central plaza">
</p>

```html
<img src="village/village.gif" alt="My commit village">
```

## Getting started
Set up the nightly workflow in [Getting started](docs/getting-started.md). The composite Action downloads the binary published for each release; the included workflow installs from source instead.

```sh
cargo install --path .
git-village collect octocat --output snapshot.json
git-village render snapshot.json --output-dir village
```

Collection requires an authenticated [GitHub CLI](https://cli.github.com/). The tool outputs `village.gif`, `village.png`, and `preview.html`. Exports have transparent backgrounds and a 4px native margin to blend into GitHub light and dark modes. Adding `--background` scene draws the full sky, ocean, and retro status strip at a minimum 768 × 512 resolution. The renderer automatically tunes down frame motion to hold the GIF under 1 MB, and maximum city growth caps out around 600,000 commits.

## City Simulation
* Growth: All-time public commits expand the grid outward from the central plaza, block by block.

* Palette: Buildings take on colors from active languages, mixed naturally across every neighborhood.

* Movement: Yearly commits set population density, monthly commits drive traffic flow, and weekly commits keep construction cranes active.

* Status strip: The scene view includes a retro HUD displaying handle, town stage, total commits, population count, daily mood, and language swatches.

See [the design](docs/design.md) for thresholds and mood rules.

Choose a look with `--setting`, or the Action's `setting` input. Earth is the default. Like the stills further down, these come from a sample account and use `--background scene`, with the status strip cropped out.

## Settings

Switch biomes with `--setting` or the Action's `setting` input:

```sh
git-village render snapshot.json --output-dir village --setting mars
```

<table>
  <tr>
    <td align="center"><img src="docs/gallery/setting-earth.png" alt="Earth: a grass island in a blue sea with sailboats"><br><code>earth</code></td>
    <td align="center"><img src="docs/gallery/setting-sky.png" alt="Sky: a floating island over a sea of cloud"><br><code>sky</code></td>
    <td align="center"><img src="docs/gallery/setting-mars.png" alt="Mars: a red island with boulders and dust devils under a butterscotch sky"><br><code>mars</code></td>
  </tr>
  <tr>
    <td align="center"><img src="docs/gallery/setting-historic.png" alt="Historic: timber buildings and fields at a warm dusk"><br><code>historic</code></td>
    <td align="center"><img src="docs/gallery/setting-futuristic.png" alt="Futuristic: a neon grid under a violet sky with a distant skyline"><br><code>futuristic</code></td>
    <td align="center"><img src="docs/gallery/setting-underwater.png" alt="Underwater: a sand floor with kelp and shafts of light"><br><code>underwater</code></td>
  </tr>
  <tr>
    <td align="center"><img src="docs/gallery/setting-underground.png" alt="Underground: a dark cavern with purple crystals and stalactites"><br><code>underground</code></td>
    <td></td>
    <td></td>
  </tr>
</table>

A village grows through six stages as lifetime commits add up.

<table>
  <tr>
    <td align="center"><img src="docs/gallery/outpost.png" alt="Outpost: a small grey island with a tent"><br>Outpost, 0 commits</td>
    <td align="center"><img src="docs/gallery/hamlet.png" alt="Hamlet: a small green island with a few buildings"><br>Hamlet, 100 commits</td>
    <td align="center"><img src="docs/gallery/village.png" alt="Village: a round island with its first blocks around the plaza"><br>Village, 1,500 commits</td>
  </tr>
  <tr>
    <td align="center"><img src="docs/gallery/town.png" alt="Town: a wider island of mixed shops and houses"><br>Town, 5,000 commits</td>
    <td align="center"><img src="docs/gallery/city.png" alt="City: dense blocks with towers rising near the center"><br>City, 20,000 commits</td>
    <td align="center"><img src="docs/gallery/metropolis.png" alt="Metropolis: a large island with a tall downtown"><br>Metropolis, 100,000 commits</td>
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

## Development
```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run --release --example generate-sheet -- target/review
```
The review generator outputs stage GIFs and visual contact sheets. The core Rust library runs deterministically across independent modules for collection, snapshot validation, development rules, mood evaluation, setting assets, layout, depth rasterization, motion, and image encoding.

## Credits & License

Inspired by [git-koi](https://github.com/yalongwastaken/git-koi), by Anthony Yalong. Adapted code and design conventions appear in [NOTICE](NOTICE).

Distributed under the MIT license, copyright Rei Tamaru. See [LICENSE](LICENSE).
