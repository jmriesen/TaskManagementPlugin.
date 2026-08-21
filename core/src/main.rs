use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use core::document::Document;
use core::{combine_requires, filter_todos, TagFilter};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Search for todo items, optionally filtering by tags
    Todos {
        dir_path: PathBuf,

        /// All of these
        #[arg(short, long)]
        include: Vec<String>,

        // At least one of these
        #[arg(short, long)]
        any: Vec<String>,

        // None of these
        #[arg(short, long)]
        exclude: Vec<String>,
    },
    /// Find all #Requires<Name> tags and group results by person
    Requires { dir_path: PathBuf },
    /// Find files where the filename and the h1 heading don't match
    HeadingCheck { dir_path: PathBuf },
    /// Collect all tags across every .md file, sorted and deduplicated (cached)
    Tags { dir_path: PathBuf },
    /// Clear the tag cache so the next `tags` call rescans the vault
    ClearTagCache { dir_path: PathBuf },
}

fn main() {
    match Cli::parse().command {
        Command::Todos {
            dir_path,
            include,
            any,
            exclude,
        } => {
            let filter = TagFilter {
                include: include.iter().map(|t| format!("#{t}")).collect(),
                any: any.iter().map(|t| format!("#{t}")).collect(),
                exclude: exclude.iter().map(|t| format!("#{t}")).collect(),
            };
            let todos = find_paths(&dir_path).flat_map(|p| Document::from_file(&p).todos());
            let results = filter_todos(todos, &filter);
            print!(
                "{}",
                serde_json::to_string_pretty(&results).expect("serialization should never fail.")
            );
        }
        Command::HeadingCheck { dir_path } => {
            let results: Vec<_> = find_paths(&dir_path)
                .filter_map(|p| Document::from_file(&p).heading_mismatch())
                .collect();
            print!(
                "{}",
                serde_json::to_string_pretty(&results).expect("serialization should never fail.")
            );
        }
        Command::Tags { dir_path } => {
            let cache = dir_path.join(".tag_cache.json");
            if cache.exists() {
                print!("{}", std::fs::read_to_string(&cache).expect("failed to read cache"));
            } else {
                let mut tags: Vec<String> = find_paths(&dir_path)
                    .flat_map(|p| Document::from_file(&p).tags())
                    .collect();
                tags.sort();
                tags.dedup();
                let json = serde_json::to_string_pretty(&tags).expect("serialization should never fail.");
                std::fs::write(&cache, &json).expect("failed to write cache");
                print!("{}", json);
            }
        }
        Command::ClearTagCache { dir_path } => {
            let cache = dir_path.join(".tag_cache.json");
            if cache.exists() {
                std::fs::remove_file(&cache).expect("failed to delete cache");
            }
        }
        Command::Requires { dir_path } => {
            let documents = find_paths(&dir_path).map(|p| Document::from_file(&p));
            let combined = combine_requires(documents);
            print!(
                "{}",
                serde_json::to_string_pretty(&combined).expect("serialization should never fail.")
            );
        }
    }
}

fn find_paths(dir: &Path) -> impl Iterator<Item = PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    std::iter::from_fn(move || {
        while let Some(path) = stack.pop() {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&path) {
                    stack.extend(entries.filter_map(|e| e.ok()).map(|e| e.path()));
                }
            } else if path.extension().is_some_and(|e| e == "md") {
                return Some(path);
            }
        }
        None
    })
}
