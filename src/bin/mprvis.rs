extern crate mprvis;

use std::thread;

fn main() {
    let gui = thread::spawn(|| {
        let gui = mprvis::gui::GUI::new(800, 600);
        gui.run_loop();
    });

    let mpris = thread::spawn(|| {
        let conn = mprvis::init();
        println!("{:?}", mprvis::metadata::get_current(&conn));
    });
    
    gui.join().unwrap_or_else(|err| {
        println!("GUI panicked: {:?}", err);
    });
    mpris.join().unwrap_or_else(|err| {
        println!("MPRIS panicked: {:?}", err);
    });
}
