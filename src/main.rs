use std::{
    env,
    path::{Path, PathBuf},
};

use search::find_todos;

fn main() {
    let mut args = env::args();
    let _program_name = args.next();

    let dir_name = args.next().expect("You must pass in a file to search");
    let dir_path = &PathBuf::from(dir_name.as_str());

    let results: Vec<_> = find_paths(dir_path)
        .iter()
        .map(|x| find_todos(x))
        .flatten()
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
