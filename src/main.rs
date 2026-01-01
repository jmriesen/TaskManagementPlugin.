use std::env;

use search::find_next_actions;

fn main() {
    let mut args = env::args();
    let _program_name = args.next();
    let file_name = args.next().expect("You must pass in a file to search");
    let file = std::fs::File::open(dbg!(file_name)).expect("Error opening file");
    let results = find_next_actions(file);
    print!(
        "{}",
        serde_json::to_string_pretty(&results).expect("seralization should never fail.")
    );
}
