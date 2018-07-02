extern crate dbus;
#[macro_use]
extern crate conrod;

use self::dbus::{Connection, BusType};

pub fn init() -> Connection {
    Connection::get_private(BusType::Session).unwrap()
}

pub mod mpris;
pub mod gui;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
