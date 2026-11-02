use git_village::DISPLAY_NAME;
use git_village::{
    collect::{GhCli, collect_account_snapshot},
    output,
    render::{Backdrop, Renderer},
    setting::Setting,
    snapshot::Snapshot,
};
use std::{collections::BTreeMap, path::Path};
fn usage() -> ! {
    eprintln!(
        "Usage: {DISPLAY_NAME} collect LOGIN [--output FILE]\n       {DISPLAY_NAME} render SNAPSHOT --output-dir DIR [--background none|scene] [--setting NAME]"
    );
    std::process::exit(2)
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.iter().any(|a| a.contains('/'))
        && arguments.first().is_some_and(|a| a == "collect")
        && arguments.get(1).is_some_and(|a| a.contains('/'))
    {
        return Err(format!(
            "{DISPLAY_NAME} draws one person's account; pass a login instead of owner/repository"
        )
        .into());
    }
    match arguments.as_slice() {
        [flag] if flag == "--help" => {
            println!(
                "{DISPLAY_NAME} collect LOGIN [--output FILE]\n{DISPLAY_NAME} render SNAPSHOT --output-dir DIR [--background scene|none] [--setting NAME]\nSettings: earth, sky, mars, historic, futuristic, underwater, underground"
            );
        }
        [flag] if flag == "--version" => {
            println!(
                "{} {}",
                git_village::DISPLAY_NAME,
                env!("CARGO_PKG_VERSION")
            )
        }
        [command, login, rest @ ..] if command == "collect" && !login.starts_with('-') => {
            let destination = match rest {
                [] => None,
                [flag, file] if flag == "--output" => Some(file),
                _ => usage(),
            };
            let snapshot =
                collect_account_snapshot(&GhCli, login, chrono::Utc::now().date_naive())?;
            let bytes = serde_json::to_vec_pretty(&snapshot)?;
            if let Some(file) = destination {
                let path = Path::new(file);
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or("invalid output file")?;
                output::replace_files(
                    path.parent()
                        .filter(|p| !p.as_os_str().is_empty())
                        .unwrap_or(Path::new(".")),
                    &BTreeMap::from([(name, bytes)]),
                )?;
            } else {
                println!("{}", String::from_utf8(bytes)?);
            }
        }
        [command, input, rest @ ..] if command == "render" && !input.starts_with('-') => {
            let mut destination = None;
            let mut backdrop = Backdrop::None;
            let mut setting = Setting::Earth;
            let mut chunks = rest.chunks_exact(2);
            for pair in &mut chunks {
                match pair[0].as_str() {
                    "--output-dir" if destination.is_none() => destination = Some(&pair[1]),
                    "--background" if pair[1] == "scene" => backdrop = Backdrop::Scene,
                    "--background" if pair[1] == "none" => backdrop = Backdrop::None,
                    "--setting" => {
                        setting = Setting::parse(&pair[1]).ok_or_else(|| {
                            let names: Vec<_> = Setting::ALL.iter().map(|s| s.name()).collect();
                            format!("unknown setting {}; choose {}", pair[1], names.join(", "))
                        })?;
                    }
                    _ => usage(),
                }
            }
            if !chunks.remainder().is_empty() {
                usage()
            }
            let directory = destination.unwrap_or_else(|| usage());
            let snapshot = Snapshot::from_json(&std::fs::read(input)?)?;
            let renderer = Renderer::with_backdrop(&snapshot, setting, backdrop)?;
            let (frames, gif) = output::encode_within_budget(renderer)?;
            println!("{} frames, {} GIF bytes", frames.len(), gif.len());
            output::replace_files(
                Path::new(directory),
                &BTreeMap::from([
                    (git_village::GIF_FILE, gif),
                    (
                        git_village::PNG_FILE,
                        output::encode_png_with_bounds(&frames[0], output::export_bounds(&frames))?,
                    ),
                    ("preview.html", output::encode_preview_html().into_bytes()),
                ]),
            )?;
        }
        _ => usage(),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{DISPLAY_NAME}: {error}");
        std::process::exit(1)
    }
}
