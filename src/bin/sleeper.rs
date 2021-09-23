use std::{env, fs};

use signal_hook::{consts::signal::SIGTERM, iterator::Signals};

// Copy itself to given path. Otherwise, wait for SIGTERM.
fn main() -> Result<(), std::io::Error> {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        let mut signals = Signals::new(&[SIGTERM])?;
        for signal in signals.forever() {
            match signal {
                SIGTERM => break,
                _ => unreachable!(),
            }
        }
    } else {
        fs::copy(&args[0], &args[1])?;
    }
    Ok(())
}
