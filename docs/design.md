# Design

Git village draws a living pixel-art city from one developer's commits, for that person's own GitHub profile README. It is a personal portrait and not a shared world: there is no community map, no other users' buildings, and no repository mode. It follows the shape of [git-koi](https://github.com/yalongwastaken/git-koi): a one-way pipeline, a nightly Action that commits a looping GIF to a profile README, a deterministic core that reads no clock and does no I/O, and honest data where every visual trait traces back to a number.

## What the city means

The city is a reading of the data, so each trait has one source and the same snapshot always draws the same city.

| GitHub signal | In the city | Rule |
| --- | --- | --- |
| Account ID | Identity | Seeds the terrain, the plot order, and agent routes, so a city keeps its shape from day to day |
| Lifetime commits | Growth stage, hub area, hub building count and tallest height | Bounded logarithmic growth with diminishing gains |
| Commits in the latest 365 days | Population | One resident per commit. The number is shown as is, and walkers on screen follow it on a log curve, so a busy year fills the streets and a quiet year empties them |
| Lifetime commits per language | Language peninsulas and their buildings | At 25 commits a language earns one of eight permanent arms; its own count controls its land and plots on bounded power curves. Hub languages receive their own small frontage counts; remaining hub buildings are neutral civics |
| Language family | Building silhouette | See archetypes below |
| Commits in the latest 30 days | Traffic | How many vehicles are on the streets, on a log curve, so a busy month jams the grid and a quiet month empties it |
| Commits in the latest 7 days | Construction sites | Number of cranes and scaffolded buildings |
| Current weekly streak | Upkeep | Share of lit windows and street lamps, fading after a streak ends |
| Longest weekly streak | Monument | Central landmark that gains tiers and never shrinks |
| The day that just ended | The city's mood | See moods below |

Growth is append-only. Plots are claimed in a fixed seed-derived order, so more commits add buildings and leave standing ones where they are. Hub buildings use each minor language's own count, and civic buildings take over the hub capacity retained when a language earns its arm. Arm plots keep their language, color and archetype; only office Tower archetypes become tall at advanced stages.

### Growth stages

| Stage | Commits from | Feel |
| --- | --- | --- |
| Outpost | 0 | A tent or two and a campfire on an empty plot |
| Hamlet | 25 | A short lane of small houses |
| Village | 250 | A crossroads and a plaza |
| Town | 1,000 | Low, varied shop and house frontages along each avenue |
| City | 5,000 | Dense blocks and a full street grid |
| Metropolis | 20,000 | Tall office districts and busy street frontages with visible avenues |

A zero total is a real state and draws the Outpost, never an error. The overall stage thresholds and logarithmic height signal remain unchanged: 5, 7, 12, 22, 43, and 74 native pixels at the six anchors, reaching 80 at 100,000 commits. Actual building art follows its archetype. Town buildings, including roof ornaments, stay below twice their footprint width. Shops and depots are low and broad, foundries have short chimneys, academies have domes, and office towers gain height only at City and Metropolis. Office heights taper along the avenues, and projection caps keep roofs and motion inside the canvas.

Each language requests `round((commits / 15)^0.78)` buildings, with a minimum of one for positive commits and a cap of 96 before available plots limit it. The same rule applies to languages living in the hub. Neutral civic demand follows the overall log signal, with retained capacity for each earned arm so existing hub plots do not disappear. Civic buildings are low muted stone, around a broader tiered monument. The real 1,697-commit review fixture has one building each for C++, JavaScript, Jupyter Notebook and Processing; its arm frontages are strictly ordered by commits.

### Land and plots

`CityPlan` derives all land, surfaces, plots and routes purely from a snapshot. A compact, roughly round hub centered at world (56, 56) grows from radius 30 to 32 on a log curve. Each qualifying language grows a long peninsula with a narrow root and a rounded tip, leaving deep empty coves between districts. Edge perturbations depend only on the account seed and position, so increasing counts adds land without removing old tiles. A sand rim follows missing neighbors. Soil is a solid tile volume, with complete depth-tested faces under exposed front edges; shared corner pixels close the former transparent comb stripes.

Arm slots still follow the date a language reached 25 lifetime commits, with name ties. Their fixed screen order is east, west, southeast, southwest, northeast, northwest, south, north. Collection sums per-language daily counts in a sorted map before finding this date, independently of fetch and split order. At most eight languages receive arms; other languages and unclassified commits belong to the hub. Arms use their own commits, never their share or another language's total.

Physical arm length initially follows `72 * (commits / 100)^0.4`, up to 92 percent of its direction's maximum. A small log tail from 1,000 to 100,000 commits reaches the remaining eight percent, preserving visible growth after the early power curve. Physical maxima are 130 world units east and west, 150 in diagonal screen directions, and 115 north and south. Full widths reach 36, 50 and 36 respectively on a bounded power curve. Arms have length at least 2.5 times their width from 100 commits onwards. In the real fixture TypeScript's arm is at least 1.3 times Python's length and 1.6 times its land area; a circle through the mid-arm region crosses four separate land intervals.

Fixed, closely spaced plot rows line both sides of each avenue, with seed-shuffled side order inside each distance band. Arm footprints are six or seven world units, with small gaps, and civic footprints are four. Hub and arm ownership have a permanent gap; future streets are reserved before claims. Candidate order and geometry are independent of the snapshot's language array order.

### Archetypes

Language picks a silhouette, the setting picks its materials. Unknown languages take a stable archetype from a hash of their name.

| Archetype | Languages | Silhouette and motion |
| --- | --- | --- |
| Foundry | Rust, C, C++, Zig, Assembly, Fortran | Sawtooth roof, chimneys with rising smoke |
| Academy | Python, R, Julia, Jupyter Notebook, MATLAB, TeX | Dome and columns, slow rotating observatory slit |
| Market | JavaScript, TypeScript, HTML, CSS, Vue, Svelte, PHP | Low shops, striped awnings, blinking signs |
| Tower | Java, C#, Go, Kotlin, Swift, Scala, Dart | Tall office slab, roof beacon |
| Depot | Shell, Dockerfile, Nix, HCL, Makefile, PowerShell, Lua | Wide warehouse, loading door, stacked crates |
| Temple | Haskell, Lisp, OCaml, Elixir, Erlang, Clojure, Ruby | Tiered roof and spire, pennant |

### Moods

One mood per run, chosen by ordered rules on the day that just ended. The first match wins, using git-koi's thresholds, and a quiet day is a day without commits. Commit milestones are 100, 500, 1,000, 5,000, 10,000, 25,000, and 50,000.

| Mood | When the observation date | The city |
| --- | --- | --- |
| Abandoned | ends 60 or more quiet days | Almost nobody about, dark windows, nature reclaiming the plot |
| Night | ends 14 or more quiet days | A night palette, a few walkers, a few warm windows |
| Market day | is the first day with commits in its Monday-Sunday week | Crowd gathers at the plaza, stalls appear |
| Festival | falls within 7 days of crossing a milestone, or is a Sunday finishing a busy week | Crowded streets, bursts in the sky |
| Workday | is anything else | Steady commuters and traffic |

## Settings

The first release has one setting, `earth`, and no `--setting` flag, so the look can be judged and refined before it is multiplied. The renderer reads colors, materials, and vehicle skins from a setting data structure instead of hard-coding them, so another setting can be added later without touching layout or motion. The rows other than `earth` below are the plan for later and are not built yet. A setting changes what the city looks like and how nature behaves, never what the data says.

| Setting | Ground and backdrop | Materials | Vehicles | Nature in motion |
| --- | --- | --- | --- | --- |
| `earth` (default) | Grass island, sandy shore, sea, soft sky | Brick, plaster, tile | Cars | Drifting clouds, flocking birds, swaying trees |
| `sky` | Floating island with rock underside, cloud sea | White stone, brass | Airships | Clouds passing under and over, waterfall off the edge |
| `mars` | Red regolith, butterscotch sky, twin moons | Pressure domes, habitat metal | Rovers | Dust devils, blowing dust |
| `historic` | Cobbles and fields, warm dusk | Timber, stone, thatch | Horse carts | Smoke, flags, grazing sheep |
| `futuristic` | Dark plaza with neon grid, deep violet sky | Glass, chrome, neon trim | Hover cars on sky lanes | Blinking holograms, drifting lights |
| `underwater` | Sand floor, blue gradient, light shafts | Coral, glass bubbles | Small submarines | Fish schools, rising bubbles, swaying kelp |
| `underground` | Cavern floor, stalactites, darkness at the edges | Cut stone, timber props, lanterns | Mine carts | Glinting crystals, water drips, bats |

## Rendering

Isometric voxels at the 2:1 pixel-art projection. Isometric reads as a city builder and can carry the planned settings, and voxels give consistent three-tone shading and correct occlusion without hand-drawn sprites.

The working canvas is 384 by 256 pixels and exports retain the same doubled pixel scale. Transparent exports crop to the union of all opaque loop pixels plus four native pixels of margin on every side. Every GIF frame and its PNG still use the same rectangle. Scene mode keeps the full 768 by 512 export. Embed with `<img src="village/village.gif" alt="Animated commit village">` and no width attribute, so the sticker displays at its exported pixel size; the preview page also leaves the width intrinsic.

Each frame composes two passes. The static pass rasterizes the backdrop, terrain, and buildings once into a color buffer and a depth buffer. The dynamic pass draws people, vehicles, smoke, and nature for each frame, tested against that depth, so a walker disappears behind a tower without any sorting of whole objects.

The city sits on a sandy island in a shaded sea, with distant coast, periodic wave glints, and sailboats. Dirt circuits and lanes become paved at Village and gain full width, markings and sidewalks from Town. The hub ring connects to every arm's spine, which closes around successive blocks. Existing block cross streets stay in place as avenues extend, and larger stages add and widen hub cross streets. People follow short sidewalk block loops in each arm plus a plaza loop; extra Market day and Festival walkers gather at the plaza. The elevated railway is one slim straight east-west line through the hub and the roots of the first two arms, with widely spaced pillars.

By default the island stands on a slab of earth on a transparent canvas, the way git-koi sits on the page, so it suits GitHub's light and dark themes alike. `--background scene` adds the sky, sea, boats, birds, and status strip. A transparent GIF stores a base frame of every unchanging pixel, then each loop frame as its differences from that base with Previous disposal, because Keep disposal cannot erase a pixel back to the page.

A three-row status strip along the bottom, in a 3-by-5 pixel font, shows login, stage, mood, lifetime commits, and population with thousands separators. Counts share one line when they fit and use two lines for very large values. Night has its own sky, moon, twinkling stars and warm window lights applied after material shading; Abandoned uses an overcast palette, desaturated buildings, weeds, and no vehicles. Festival has expanding, fading bursts just above the city skyline and within its horizontal extent in transparent mode, plus plaza bunting, while Market day uses colored canopy stalls at the foreground plaza.

## Motion

The GIF loops, so every moving thing must close exactly. Each motion is a function of `frame_index / frame_count` and completes a whole number of cycles per loop. Nothing accumulates state between frames, and a test compares frame `n` against what frame `n` would be one full loop later.

- **People** are two-pixel dots with a one-frame step bob, walking closed routes around blocks. Their number comes from population on a log curve and the mood's multiplier. Their colors come from a small clothing palette.
- **Mobility** unlocks with growth and each mode adds to the ones before it, so the transport system tells the city's age at a glance.

| Stage | New mode | Behavior |
| --- | --- | --- |
| Outpost | Handcart | One slow cart on the track |
| Hamlet | Bicycles | Quick riders at the road's edge |
| Village | Cars | Several colors, driving on the right in their own lane |
| Town | Buses | Long and slow, pausing at marked stops |
| City | Trams | Multi-car, on visible rails along one main avenue |
| Metropolis | Elevated train | Multi-car on a viaduct on pillars above the streets, casting a shadow |

Vehicles share out the traffic count across the unlocked modes. Ground vehicles follow closed integer polylines sampled by exact Manhattan arc length, one lap per loop, with shared intersection dwell to preserve lane spacing. Buses pause beside blue stop signs. Trams use visible rails on the hub ring; the elevated train reverses on its separate straight line. Traffic fills every district before adding vehicles to a lane; each circuit has length-based capacity and equal phase spacing, with opposite stretches of the circuit on separate sides. Monthly activity caps traffic at five vehicles per unlocked stage, reaching 30 at Metropolis. Active population grows towards 46 walkers, with Festival adding 14. Vehicles in one lane keep a gap and never overlap. People stay on sidewalks and vehicles stay on roadways.
- **Buildings** move through attached proportionate cranes with swinging arms and hoisted blocks, twinkling windows, rising smoke, beacons, and signs. Weekly commits select zero cranes at zero work, one at 1–9, two at 10–49, three at 50–499, and four from 500 at City and above. Cities with more than 80 buildings use one smoke puff per foundry instead of three, keeping the dense Festival under the GIF budget without changing its buildings, agents, palette, or disposal scheme.
- **Nature** comes from the setting's table above. Flocks and fish schools are a center path plus per-member periodic offsets, which looks like flocking and still closes exactly.

Day and night never cycle inside a loop, because repainting every pixel each frame would make the GIF huge. Night is a whole-loop palette chosen by mood.

## Size budget

A profile GIF has to stay small. The default loop is 288 frames at 12 frames per second, for 24 seconds, so the city moves at an unhurried pace. Frames use one shared global palette of at most 256 colors, built from every color the loop actually uses, and each frame after the first stores only the box around pixels that changed, with unchanged pixels transparent. The budget is 4 MiB for the worst case, a Metropolis festival, enforced by a test.

## Commit data

```sh
git-village collect octocat --output snapshot.json
git-village render snapshot.json --output-dir village
```

The city counts public commits only, whatever the token can see. A personal `gh` token and a workflow's built-in token must draw the same city, and private work must never show on a public profile, so a private repository is filtered out on its `visibility` before anything is counted. That is why totals are summed from public repositories and `totalCommitContributions` is not used: it includes private commits when the token can see them.

The snapshot language field `arm_earned_on` stores the optional threshold date; schema version remains 1. Counts come from GitHub's `contributionsCollection`, read in windows of at most 90 days from the account's creation through the last complete UTC day. Each window's `commitContributionsByRepository` gives per-day counts and the repository's primary language, and those day lists never need paging because a 90-day window has fewer than the 100-node limit. Issues, pull requests, and reviews do not count.

GitHub returns at most 100 repositories per window. A window at that limit is split in half and requested again until it falls under the limit, so the counts stay exact. A single day still at the limit is kept with what was returned and the snapshot records it as truncated. The snapshot stores totals by day and by language and never repository names.

## Pipeline

```text
collect → snapshot → development → mood → layout → render → output
```

| Module | Owns |
| --- | --- |
| `collect` | Snapshots through the GitHub CLI, behind a client trait so tests use fixtures |
| `snapshot` | The versioned JSON schema and its validation |
| `development` | Commit signals: growth, population, language shares, streaks, recent activity |
| `mood` | The ordered mood rules |
| `setting` | Earth palette, materials, vehicle skins, and nature; other settings remain future plans |
| `layout` | Language archetypes, colors and deterministic hashing |
| `plan` | Snapshot-owned tile masks, permanent districts, claimed buildings, surfaces and polylines |
| `voxel` | The isometric rasterizer, depth buffer, and frame type |
| `motion` | Loop constants and agents' poses as pure functions of the frame |
| `render` | Composes a still or a loop frame, including the status strip |
| `output` | PNG, GIF, and the preview page |

Everything after `collect` is deterministic, reads no clock, and performs no I/O. Callers own files. Files are written through a temporary sibling only after every output has rendered, so a failed run keeps yesterday's city.
