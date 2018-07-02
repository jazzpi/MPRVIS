extern crate mprvis;

use std::thread;

fn main() {
    let gui = thread::spawn(|| {
        let mut gui = mprvis::gui::GUI::new(800, 600);
        gui.run_loop();
    });

    gui.join().unwrap_or_else(|err| {
        println!("GUI panicked: {:?}", err);
    });
}
