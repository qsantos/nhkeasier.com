use std::time::Instant;

use edict2::SubEdictCreator;

fn main() {
    let sub_edict_creator = SubEdictCreator::from_files().expect("failed to load EDICT2");

    let story_in_input = 1000;
    let iterations = 10;
    let start = Instant::now();
    let data = include_str!("../tests/test-input");
    for _ in 0..iterations {
        sub_edict_creator.from(data);
    }
    let elapsed = start.elapsed();
    let elapsed_per_iteration = elapsed / iterations;
    eprintln!(
        "{story_in_input} in {:?}, {:?} per story",
        elapsed_per_iteration,
        elapsed_per_iteration / story_in_input
    );
}
