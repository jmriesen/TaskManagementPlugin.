use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use core::{find_requires, find_todos};

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
}

fn main() {
    match Cli::parse().command {
        Command::Todos {
            dir_path,
            mut include,
            mut any,
            mut exclude,
        } => {
            let add_leading_hash = |tags: &mut Vec<String>| {
                tags.iter_mut().for_each(|i| i.insert(0, '#'));
            };
            add_leading_hash(&mut include);
            add_leading_hash(&mut any);
            add_leading_hash(&mut exclude);

            let results: Vec<_> = find_paths(&dir_path)
                .iter()
                .map(|x| find_todos(x))
                .flatten()
                .filter(|x| include.is_empty() || include.iter().all(|y| x.tags.contains(y)))
                .filter(|x| any.is_empty() || any.iter().any(|y| x.tags.contains(y)))
                .filter(|x| exclude.iter().all(|y| !x.tags.contains(y)))
                .collect();
            print!(
                "{}",
                serde_json::to_string_pretty(&results).expect("serialization should never fail.")
            );
        }
        Command::Requires { dir_path } => {
            let mut combined: std::collections::HashMap<String, Vec<core::SearchResult>> =
                std::collections::HashMap::new();
            for path in find_paths(&dir_path) {
                for (person, results) in find_requires(&path) {
                    combined.entry(person).or_default().extend(results);
                }
            }
            print!(
                "{}",
                serde_json::to_string_pretty(&combined).expect("serialization should never fail.")
            );
        }
    }
}

fn find_paths(dir: &Path) -> Vec<PathBuf> {
    let dir = std::fs::read_dir(dir).expect("error opening directory");
    dir.filter_map(|x| x.ok())
        .map(|x| {
            let file_type = x.file_type().unwrap();
            if file_type.is_dir() {
                find_paths(&x.path())
            } else if file_type.is_file() {
                vec![x.path()]
            } else {
                vec![]
            }
        })
        .flatten()
        .collect()
}
