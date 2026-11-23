use mkxelz_core::{generate, presets, Spec};
use std::path::PathBuf;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(|s| s.as_str()) {
        Some("gen") => cmd_gen(&args[1..], true),
        Some("validate") => cmd_gen(&args[1..], false),
        Some("presets") => {
            for (name, _) in presets::PRESETS {
                println!("{name}");
            }
            Ok(())
        }
        Some("--version") | Some("-V") => {
            println!("mkxelz {}", mkxelz_core::VERSION);
            Ok(())
        }
        _ => {
            print_help();
            Ok(())
        }
    }
}

fn print_help() {
    println!(
        "mkxelz {}\n\n\
         USAGE:\n  \
         mkxelz gen <spec.json | preset:NAME> [--out DIR] [--seed N] [--size PX] [--set name=true|false]...\n  \
         mkxelz validate <spec.json | preset:NAME>\n  \
         mkxelz presets\n  \
         mkxelz --version\n",
        mkxelz_core::VERSION
    );
}

fn cmd_gen(args: &[String], write: bool) -> Result<(), Box<dyn std::error::Error>> {
    let mut source: Option<String> = None;
    let mut out_dir = PathBuf::from("mkxelz-out");
    let mut seed: Option<u64> = None;
    let mut size: Option<u32> = None;
    let mut sets: Vec<(String, bool)> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--out" => {
                i += 1;
                out_dir = PathBuf::from(args.get(i).ok_or("--out needs a value")?);
            }
            "--seed" => {
                i += 1;
                seed = Some(args.get(i).ok_or("--seed needs a value")?.parse::<u64>()?);
            }
            "--size" => {
                i += 1;
                size = Some(args.get(i).ok_or("--size needs a value")?.parse::<u32>()?);
            }
            "--set" => {
                i += 1;
                let kv = args.get(i).ok_or("--set needs name=true|false")?;
                let (k, v) = kv.split_once('=').ok_or("--set expects name=true|false")?;
                sets.push((k.to_string(), v.parse::<bool>()?));
            }
            other if other.starts_with("--") => {
                return Err(format!("unknown option '{other}'").into());
            }
            other => source = Some(other.to_string()),
        }
        i += 1;
    }

    let src = source.ok_or("missing spec: a .json path or preset:NAME")?;
    let text = if let Some(name) = src.strip_prefix("preset:") {
        presets::get(name)
            .ok_or_else(|| format!("unknown preset '{name}'"))?
            .to_string()
    } else {
        std::fs::read_to_string(&src)?
    };

    let mut spec = Spec::from_json(&text)?;
    if let Some(s) = seed {
        spec.seed = s;
    }
    if let Some(s) = size {
        spec.tile_size = s;
    }
    for (k, v) in sets {
        spec.switches.insert(k, v);
    }

    spec.validate()?;
    if !write {
        println!("spec ok: {} ({} tiles)", spec.name, spec.tiles.len());
        return Ok(());
    }

    let out = generate(&spec)?;
    for f in out.export(&out_dir)? {
        println!("wrote {}", f.display());
    }
    Ok(())
}
