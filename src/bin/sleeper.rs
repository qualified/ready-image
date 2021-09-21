use std::{env, fs, thread, time::Duration};

// Copy itself to given path. Otherwise, sleep for 30 years.
fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        const DAY: u64 = 24 * 60 * 60;
        thread::sleep(Duration::from_secs(30 * 365 * DAY));
    } else {
        fs::copy(&args[0], &args[1]).expect("copy");
    }
}
