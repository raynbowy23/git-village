# Design

Git village draws a living pixel-art city from one developer's commits, for that person's own GitHub profile README. It is a personal portrait and not a shared world: there is no community map, no other users' buildings, and no repository mode. It follows the shape of [git-koi](https://github.com/yalongwastaken/git-koi): a one-way pipeline, a nightly Action that commits a looping GIF to a profile README, a deterministic core that reads no clock and does no I/O, and honest data where every visual trait traces back to a number.

## What the city means

The city is a reading of the data, so each trait has one source and the same snapshot always draws the same city.

| GitHub signal | In the city | Rule |
| --- | --- | --- |
| Account ID | Identity | Seeds the terrain, the plot order, and agent routes, so a city keeps its shape from day to day |
| Lifetime commits | Growth stage, island size, building count and tallest height | Grows slowly on sublinear curves, until the GIF budget is reached |
| Commits in the latest 365 days | Population | One resident per commit. The number is shown as is, and walkers on screen follow it on a log curve, so a busy year fills the streets and a quiet year empties them |
| Lifetime commits per language | Which buildings stand across the city | Each plot goes to one language, weighted by that language's commits, so languages mix through every block. Three neutral civics stand around the plaza |
| Language family | Building silhouette | See archetypes below |
| Commits in the latest 30 days | Traffic | How many vehicles are on the streets, on a log curve, so a busy month jams the grid and a quiet month empties it |
| Commits in the latest 7 days | Construction sites | Number of cranes and scaffolded buildings |
| Current weekly streak | Upkeep | Share of lit windows and street lamps, fading after a streak ends |
| Longest weekly streak | Monument | Central landmark that gains tiers and never shrinks |
| The day that just ended | The city's mood | See moods below |

Growth is append-only. Blocks open and plots are claimed in a fixed order from the center outward, so more commits add buildings and leave standing ones where they are. A plot keeps its position and footprint, and its tenant changes only when another language grows enough to overtake it; only office Tower archetypes become tall at advanced stages.

### Growth stages

| Stage | Commits from | Feel |
| --- | --- | --- |
| Outpost | 0 | A tent or two and a campfire on an empty plot |
| Hamlet | 100 | A short lane of small houses |
| Village | 1,500 | A crossroads and a plaza |
| Town | 5,000 | Low, varied shop and house frontages along each avenue |
| City | 20,000 | Dense blocks and a full street grid |
| Metropolis | 100,000 | Tall office districts and busy street frontages with visible avenues |

A zero total is a real state and draws the Outpost, never an error. The stage thresholds are fixed. The height signal is 5, 7, 12, 22, 43, and 74 native pixels at the six anchors and 80 at 500,000 commits. Actual building art follows its archetype. Town buildings, including roof ornaments, stay below twice their footprint width. Shops and depots are low and broad, foundries have short chimneys, academies have domes, and office towers gain height only at City and Metropolis. Office heights peak downtown and fall away toward the coast.

The city claims `round((commits / 60)^0.7)` buildings, with a minimum of one, so 1,000 commits is about seven buildings and 20,000 about 58. Civic buildings are low muted stone, around a broader tiered monument.

### Land and plots

`CityPlan` derives all land, surfaces, plots and routes purely from a snapshot. The land is one round island centered at world (56, 56), with a gentle coastline wobble that depends only on the account seed and the angle. Its radius follows the farthest opened block plus a margin, so increasing counts adds land without removing old tiles. A sand rim follows missing neighbors. Soil is a solid tile volume, with complete depth-tested faces under exposed front edges.

The streets are a grid with lines every 24 tiles, and the central block holds the plaza, monument and three civic buildings. Other blocks open in a permanent order, nearest the center first, with ties settled by the account seed. Each block has up to four plots, and one lot in five stays green. Street corridors are reserved at full width from the start, so a road widening at a later stage never reaches a plot. Footprints are five or six world units, with small gaps, and civic footprints are four. Candidate order and geometry are independent of the snapshot's language array order.

Each plot goes to one language by weighted rendezvous hashing, where a language's weight is `round((commits / 15)^0.78)`. An active language therefore shows up across the city in rough proportion to its commits, and a tenant depends only on the account, the plot and the weights. `arm_earned_on` remains in the snapshot for schema compatibility, and the plan no longer reads it.

Growth stops at 200 blocks, reached at roughly 600,000 public commits, because the still ground and buildings of a larger city would not fit the GIF budget even with almost no motion.

Open grass carries still trees in patches of dense grove, light scatter and clearing, with round trees and pines. A tree depends only on the account and its tile and goes only when something is built over it. Grove trees do not sway, which keeps them out of every GIF frame after the first.

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

`--setting` chooses among seven settings, with `earth` as the default. A setting supplies colors, materials, vehicle skins, what grows on open ground, and a little of its own motion, without touching layout or the data. Every setting draws the same plots, streets and agents from the same snapshot. Buildings keep their language's hue, mixed with a share of the setting's material so they belong there. A setting changes what the city looks like and how nature behaves, never what the data says.

| Setting | Ground and backdrop | Materials | Vehicles | Nature in motion |
| --- | --- | --- | --- | --- |
| `earth` (default) | Grass island, sandy shore, sea, soft sky | Brick, plaster, tile | Cars | Drifting clouds, birds, swaying trees |
| `sky` | Green island on a pale rock underside, sea of cloud | White stone, brass | Floating airships | Clouds, streams spilling over the edge |
| `mars` | Red regolith with boulders, butterscotch sky, twin moons | Habitat metal | Rovers | Dust devils, blowing dust |
| `historic` | Fields at a warm dusk | Timber, stone | Carts | Smoke, flags, grazing sheep |
| `futuristic` | Dark plaza with a neon grid and pylons, violet sky, distant skyline | Glass, chrome | Hovering cars | Flickering holograms, drifting lights |
| `underwater` | Sand floor with kelp, blue gradient, light shafts | Coral, glass | Small submarines | Fish schools, rising bubbles |
| `underground` | Cavern floor with crystals, stalactites, darkness | Cut stone | Mine carts | Glinting crystals, water drips, bats |

## Rendering

Isometric voxels at the 2:1 pixel-art projection. Isometric reads as a city builder and can carry every setting, and voxels give consistent three-tone shading and correct occlusion without hand-drawn sprites.

The base canvas is 384 by 256 pixels and exports keep the same doubled pixel scale. A city that outgrows it gets a larger canvas instead of smaller people and vehicles: a frame covers whatever rectangle of projected coordinates holds the island and its roofs. Transparent exports crop to the union of all opaque loop pixels plus four native pixels of margin on every side. Every GIF frame and its PNG still use the same rectangle. Scene mode exports the whole canvas, at least 768 by 512, and repeats its sky and sea across a larger one. Embed with `<img src="village/village.gif" alt="Animated commit village">` and no width attribute, so the sticker displays at its exported pixel size; the preview page also leaves the width intrinsic. GitHub scales an image wider than its page column to fit, so a very large city appears smaller there than its pixels.

Each frame composes two passes. The static pass rasterizes the backdrop, terrain, and buildings once into a color buffer and a depth buffer. The dynamic pass draws people, vehicles, smoke, and nature for each frame, tested against that depth, so a walker disappears behind a tower without any sorting of whole objects.

The city sits on a sandy island in a shaded sea, with distant coast, periodic wave glints, and sailboats. Dirt circuits and lanes become paved at Village and gain full width, markings and sidewalks from Town. The central ring meets the street grid, and larger stages add and widen cross streets inside it. Every opened block has a vehicle circuit on its inner lanes, so two neighboring circuits form a two-way street. People follow a sidewalk loop around each block plus a plaza loop; extra Market day and Festival walkers gather at the plaza. The elevated railway is one slim straight east-west line through the center, with widely spaced pillars.

By default the island stands on a slab of earth on a transparent canvas, the way git-koi sits on the page, so it suits GitHub's light and dark themes alike. `--background scene` adds the sky, sea, boats, birds, and status strip. A transparent GIF stores a base frame of every unchanging pixel, then each loop frame as its differences from that base with Previous disposal, because Keep disposal cannot erase a pixel back to the page.

A three-row status strip along the bottom, in a 3-by-5 pixel font, shows login, stage, mood, lifetime commits, and population with thousands separators. Counts share one line when they fit and use two lines for very large values. Night has its own sky, moon, twinkling stars and warm window lights applied after material shading; Abandoned uses an overcast palette, desaturated buildings, weeds, and no vehicles. Festival has expanding, fading bursts just above the city skyline and within its horizontal extent in transparent mode, plus plaza bunting, while Market day uses colored canopy stalls at the foreground plaza.

## Motion

The GIF loops, so every moving thing must close exactly. Each motion is a function of `frame_index / frame_count` and completes a whole number of cycles per loop. Nothing accumulates state between frames, and a test compares frame `n` against what frame `n` would be one full loop later.

- **People** are two-pixel dots with a one-frame step bob, walking closed routes around blocks. Their number comes from population on a log curve, the size of the city, and the mood. Their colors come from a small clothing palette.
- **Mobility** unlocks with growth and each mode adds to the ones before it, so the transport system tells the city's age at a glance.

| Stage | New mode | Behavior |
| --- | --- | --- |
| Outpost | Handcart | One slow cart on the track |
| Hamlet | Bicycles | Quick riders at the road's edge |
| Village | Cars | Several colors, driving on the right in their own lane |
| Town | Buses | Long and slow, pausing at marked stops |
| City | Trams | Multi-car, on visible rails along one main avenue |
| Metropolis | Elevated train | Multi-car on a viaduct on pillars above the streets, casting a shadow |

Vehicles share out the traffic count across the unlocked modes. Ground vehicles follow closed integer polylines sampled by exact Manhattan arc length, one lap per loop, with shared intersection dwell to preserve lane spacing. Buses pause beside blue stop signs. Trams use visible rails on the hub ring; the elevated train reverses on its separate straight line. Traffic visits every circuit, nearest the center first, before adding vehicles to a lane; each circuit has length-based capacity and equal phase spacing, with opposite stretches of the circuit on separate sides. Monthly activity sets the traffic on a log curve and the size of the city adds to it. Walkers likewise grow with the year and the city, with Festival adding 14. The GIF budget can thin both. Vehicles in one lane keep a gap and never overlap. People stay on sidewalks and vehicles stay on roadways.
- **Buildings** move through attached proportionate cranes with swinging arms and hoisted blocks, twinkling windows, rising smoke, beacons, and signs. Weekly commits select zero cranes at zero work, one at 1–9, two at 10–49, three at 50–499, and four from 500 at City and above. Cities with more than 80 buildings use one smoke puff per foundry instead of three, and past 160 buildings an even sample of them animates, because each one adds to every frame.
- **Nature** comes from the setting's table above. Flocks and fish schools are a center path plus per-member periodic offsets, which looks like flocking and still closes exactly.

Day and night never cycle inside a loop, because repainting every pixel each frame would make the GIF huge. Night is a whole-loop palette chosen by mood.

## Size budget

A profile GIF has to stay small, because it is fetched on every page view and each changed copy stays in the repository's history. The budget is 1,000,000 bytes, in both backdrops, enforced by a test. The loop is 288 frames at 12 frames per second, for 24 seconds, so the city moves at an unhurried pace. Frames use one shared global palette of at most 256 colors, built from every color the loop actually uses, and each frame stores only the box around pixels that changed.

Rendering tries the full loop first. If it is over budget, it renders again with fewer walkers, vehicles and animated buildings, stepping through fixed levels and stopping at the first that fits, so the same snapshot always gives the same GIF. The buildings and land never change for the budget. Transparent loops are the costly case, about 0.6 MB at 1,700 commits and 2.6 MB unthinned at 20,000, because Previous disposal stores every moving pixel in every frame. Scene loops stay under 0.5 MB for most of the range.
