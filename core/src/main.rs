use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use core::document::Document;
use core::tag_filters::prefix_tags;
use core::{combine_requires, filter_todos, parse_tag_filters, TagFilter, TAG_FILTERS_FILE};

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
    /// Search for todo items with a named filter from the vault's task-filters.md
    SearchFilter { dir_path: PathBuf, name: String },
    /// List the names of the filters in the vault's task-filters.md
    Filters { dir_path: PathBuf },
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
                include: prefix_tags(include),
                any: prefix_tags(any),
                exclude: prefix_tags(exclude),
            };
            search_todos(&dir_path, &filter);
        }
        Command::SearchFilter { dir_path, name } => {
            search_todos(&dir_path, &named_filter(&dir_path, &name));
        }
        Command::Filters { dir_path } => {
            let names: Vec<_> = load_filters(&dir_path).into_keys().collect();
            print_json(&names);
        }
        Command::HeadingCheck { dir_path } => {
            let results: Vec<_> = find_paths(&dir_path)
                .filter_map(|p| Document::from_file(&p).heading_mismatch())
                .collect();
            print_json(&results);
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
            print_json(&combined);
        }
    }
}

fn search_todos(dir: &Path, filter: &TagFilter) {
    let todos = find_paths(dir).flat_map(|p| Document::from_file(&p).todos());
    print_json(&filter_todos(todos, filter));
}

fn print_json(value: &impl serde::Serialize) {
    print!(
        "{}",
        serde_json::to_string_pretty(value).expect("serialization should never fail.")
    );
}

/// Print `message` and exit, for errors the user should see without a panic's
/// backtrace noise (nvim shows the CLI's output in a notification).
fn fail(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1)
}

/// The vault's named filters, or none if it has no config file, as in Obsidian.
fn load_filters(dir: &Path) -> BTreeMap<String, TagFilter> {
    let path = dir.join(TAG_FILTERS_FILE);
    if !path.exists() {
        return BTreeMap::new();
    }
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| fail(&format!("error reading {}: {e}", path.display())));
    parse_tag_filters(&source)
        .unwrap_or_else(|e| fail(&format!("Failed to parse {TAG_FILTERS_FILE}:\n{e}")))
}

fn named_filter(dir: &Path, name: &str) -> TagFilter {
    let mut filters = load_filters(dir);
    filters.remove(name).unwrap_or_else(|| {
        let names: Vec<_> = filters.keys().map(String::as_str).collect();
        fail(&format!(
            "No filter named \"{name}\" in {TAG_FILTERS_FILE}. Filters: {}",
            if names.is_empty() { "none".to_owned() } else { names.join(", ") }
        ))
    })
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

/// Every visible `.md` file under `dir`, except the tag filter config, whose
/// unchecked items would otherwise show up as todos.
fn find_paths(dir: &Path) -> impl Iterator<Item = PathBuf> {
    let config = dir.join(TAG_FILTERS_FILE);
    let mut stack = vec![dir.to_path_buf()];
    std::iter::from_fn(move || {
        while let Some(path) = stack.pop() {
            if is_hidden(&path) || path == config {
                continue;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn setup(files: &[&str]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("find_paths_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for file in files {
            let path = dir.join(file);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "").unwrap();
        }
        dir
    }

    #[test]
    fn skips_hidden_files_directories_and_filter_config() {
        let dir = setup(&[
            "visible.md",
            "task-filters.md",
            ".hidden.md",
            ".hidden_dir/nested.md",
            "visible_dir/nested.md",
            "visible_dir/.hidden_nested.md",
        ]);

        let found: HashSet<PathBuf> = find_paths(&dir).collect();

        assert_eq!(
            found,
            HashSet::from([dir.join("visible.md"), dir.join("visible_dir/nested.md")])
        );

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn filters_are_loaded_from_the_vault_config() {
        let dir = std::env::temp_dir().join(format!("load_filters_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(load_filters(&dir).is_empty());

        std::fs::write(
            dir.join(TAG_FILTERS_FILE),
            "## Next actions\n### Include\n- [x] #NextAction\n",
        )
        .unwrap();
        assert_eq!(named_filter(&dir, "Next actions").include, vec!["#NextAction"]);

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
