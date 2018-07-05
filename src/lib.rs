extern crate cairo;
extern crate dbus;
extern crate gdk;
extern crate gdk_pixbuf;
extern crate gio;
extern crate glib;
extern crate gtk;
extern crate curl;

use self::dbus::{Connection, BusType};

pub fn init() -> Connection {
    Connection::get_private(BusType::Session).unwrap()
}

pub mod mpris;
pub mod gui;
mod art;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
