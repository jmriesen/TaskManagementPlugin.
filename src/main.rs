use std::path::{Path, PathBuf};

use clap::Parser;
use search::find_todos;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    dir_path: PathBuf,

    #[arg(short, long)]
    include: Vec<String>,

    #[arg(short, long)]
    exclude: Vec<String>,
}

fn main() {
    let mut cli = Cli::parse();
    cli.include.iter_mut().for_each(|i| i.insert(0, '#'));
    cli.exclude.iter_mut().for_each(|i| i.insert(0, '#'));

    let results: Vec<_> = find_paths(&cli.dir_path)
        .iter()
        .map(|x| find_todos(x))
        .flatten()
        .filter(|x| cli.include.iter().any(|y| x.tags.contains(y)))
        .filter(|x| cli.exclude.iter().all(|y| !x.tags.contains(y)))
        .collect();
    print!(
        "{}",
        serde_json::to_string_pretty(&results).expect("seralization should never fail.")
    );
}

//Not the most efficient but should pull every file from a directory
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
