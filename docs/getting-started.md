# A city on your profile README

Generate the city in your profile repository, then embed `village/village.gif` in its README. Only public commits count, whichever token is used. A token that can read private repositories does not add private work to the city.

```html
<img src="village/village.gif" alt="My commit village">
```

Copy [examples/profile-workflow.yml](../examples/profile-workflow.yml) into your profile repository at `.github/workflows/village.yml`. It runs nightly after midnight UTC and commits the GIF when it changes. Enable Actions and allow the workflow to write repository contents. Run it manually once from the Actions tab.

The example installs from source with `cargo install --git https://github.com/raynbowy23/git-village --locked`, which needs no release but compiles on every run. Pin `--rev` to a reviewed commit for reproducible installations.

The composite [Action](../action.yml) downloads a versioned binary and verifies its SHA-256 checksum. A version tag runs the [release workflow](../.github/workflows/release.yml), which publishes those archives and checksums and then tries the Action against them. It supports Linux x86-64 and ARM64, and macOS x86-64 and ARM64. Archive names are `<package>-<version>-<target>.tar.gz`, with a sibling `.sha256` file; the archive contains the binary named after the package. Reference the Action at a release tag, such as `raynbowy23/git-village@v0.1.0`. Its inputs are `login` (defaults to the repository owner), `output-dir` (defaults to `village`), `setting` (defaults to `earth`), and `token` (defaults to `github.token`).

For local generation, install Rust 1.88 or newer and GitHub CLI, authenticate with `gh auth login`, and run:

```sh
cargo install --path .
git-village collect YOUR_LOGIN --output snapshot.json
git-village render snapshot.json --output-dir village
```

Add `--setting` to the render command to change the look: `earth`, `sky`, `mars`, `historic`, `futuristic`, `underwater`, or `underground`. Open `village/preview.html` to inspect the GIF and still. The snapshot observes the last complete UTC day. A newly created or inactive account is valid and renders an Outpost. An `owner/repository` argument is refused because this draws one person's account.

Collection windows are at most 90 days. At 100 repositories a window is split down to one day; a day still at the cap sets `detail_truncated`. Unexpected pagination fails with a repair message. Rendering validates schema version 1 and fails before replacing files if the snapshot is invalid. All outputs render in memory before temporary sibling files replace yesterday's artifacts.

Rename the package in `Cargo.toml` to change the crate, binary, and display name. Artifact names are constants in `src/lib.rs`. The Action derives its binary name and download names from the manifest; its display name is the first line of `action.yml`. Update documentation and workflow source URLs when renaming the repository.

Transparent exports use a shared loop-wide crop with four native pixels of padding, preserving the existing doubled pixel scale. Leave the image width unspecified so the sticker displays at its exported size. `--background scene` retains the full frame, at least 768 by 512. The image grows with the city up to the 1 MB GIF budget.
