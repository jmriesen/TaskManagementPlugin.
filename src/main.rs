use std::{env, path::PathBuf};

use search::find_next_actions;

fn main() {
    let mut args = env::args();
    let _program_name = args.next();
    let file_name = args.next().expect("You must pass in a file to search");
    let results = find_next_actions(&PathBuf::from(file_name.as_str()));
    print!(
        "{}",
        serde_json::to_string_pretty(&results).expect("seralization should never fail.")
    );
}
