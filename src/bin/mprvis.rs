extern crate mprvis;

use std::thread;
use std::sync::mpsc;

fn main() {
    let (tx, rx) = mpsc::channel();
    let gui = thread::spawn(|| {
        unsafe { mprvis::gui::start(800, 600, tx); }
    });

    let events_tx = rx.recv().unwrap();
    let mpris = thread::spawn(move || {
        mprvis::mpris::MPRIS::start(events_tx);
    });

    gui.join().unwrap_or_else(|err| {
        eprintln!("GUI panicked: {:?}", err);
    });
    mpris.join().unwrap_or_else(|err| {
        eprintln!("MPRIS panicked: {:?}", err);
    })
}
