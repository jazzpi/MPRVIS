extern crate mprvis;

fn main() {
    let conn = mprvis::init();
    println!("{:?}", mprvis::metadata::get_current(&conn));
}
