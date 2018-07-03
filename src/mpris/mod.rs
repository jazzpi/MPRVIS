extern crate dbus;

use self::dbus::{Connection, BusType};
use self::dbus::arg::{self, RefArg};
use self::dbus::stdintf::org_freedesktop_dbus::Properties;
use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;
use std::str::FromStr;

#[derive(Debug)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    Stopped,
}

impl FromStr for PlaybackStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Playing" => Ok(PlaybackStatus::Playing),
            "Paused" => Ok(PlaybackStatus::Paused),
            "Stopped" => Ok(PlaybackStatus::Stopped),
            _=> Err(format!("Unknown status {:?}", s)),
        }
    }
}

#[derive(Debug)]
pub enum Event {
    Data(Metadata),
    Playback(PlaybackStatus),
}

#[derive(Debug, PartialEq)]
pub struct Metadata {
    pub title: Option<String>,
    pub album: Option<String>,
    pub artist: Option<String>,
    pub featured: Option<Vec<String>>,
    pub art: Option<String>,
}

pub struct MPRIS {
    connection: dbus::Connection,
    tx: mpsc::Sender<Event>,
}

impl MPRIS {
    const SIGNAL : &'static str =
        "type='signal',sender='org.mpris.MediaPlayer2.spotify',\
         interface='org.freedesktop.DBus.Properties',\
         member='PropertiesChanged',path='/org/mpris/MediaPlayer2',\
         arg0='org.mpris.MediaPlayer2.Player'";

    pub fn start(tx: mpsc::Sender<Event>) {
        let tx = tx.clone();

        thread::spawn(move || {
            let mpris = MPRIS {
                connection: Connection::get_private(BusType::Session).unwrap(),
                tx,
            };

            if mpris.tx.send(Event::Playback(mpris.get_status())).is_err() {
                return;
            }
            if mpris.tx.send(Event::Data(mpris.get_current())).is_err() {
                return;
            }
            mpris.connection.add_match(Self::SIGNAL).unwrap();

            'main: loop {
                for ci in mpris.connection.iter(1000) {
                    if let dbus::ConnectionItem::Signal(sig) = ci {
                        if sig.headers() == (
                            dbus::MessageType::Signal,
                            Some("/org/mpris/MediaPlayer2".to_string()),
                            Some("org.freedesktop.DBus.Properties".to_string()),
                            Some("PropertiesChanged".to_string()),
                        ) {
                            if mpris.props_changed(sig).is_err() {
                                break 'main;
                            }
                        }
                    }
                }
            }
        });
    }

    fn props_changed(&self, sig: dbus::Message)
                     -> Result<(), mpsc::SendError<Event>> {
        let raw = sig.get2::<String,
                             HashMap<String, arg::Variant<Box<arg::RefArg>>>>()
            .1.unwrap();
        if let Some(status) = raw.get("PlaybackStatus") {
            self.tx.send(Event::Playback(
                PlaybackStatus::from_str(status.as_str().unwrap()).unwrap()
            ))?;
        }

        if raw.contains_key("Metadata") {
            // We could parse the message itself... But it's incredibly
            // difficult due to dbus-rs's type system, so just fetch it again
            self.tx.send(Event::Data(self.get_current()))?;
        }

        Ok(())
    }

    pub fn get_status(&self) -> PlaybackStatus {
        let player = self.connection.with_path("org.mpris.MediaPlayer2.spotify",
                                               "/org/mpris/MediaPlayer2", 500);
        let status : String = player.get("org.mpris.MediaPlayer2.Player",
                                         "PlaybackStatus").unwrap();
        PlaybackStatus::from_str(status.as_str()).unwrap()
    }

    pub fn get_current(&self) -> Metadata {
        let player = self.connection.with_path("org.mpris.MediaPlayer2.spotify",
                                               "/org/mpris/MediaPlayer2", 500);
        let metadata = player.get("org.mpris.MediaPlayer2.Player", "Metadata")
                             .unwrap();

        Self::parse_metadata(&metadata)
    }

    fn parse_metadata(raw : &HashMap<String, arg::Variant<Box<arg::RefArg>>>)
                      -> Metadata {
        let mut data = Metadata {
            title: None,
            album: None,
            artist: None,
            featured: None,
            art: None,
        };

        if let Some(title) = raw.get("xesam:title").and_then(|t| t.as_str()) {
            if title.len() > 0 {
                data.title = Some(title.to_string());
            }
        }
        if let Some(album) = raw.get("xesam:album").and_then(|t| t.as_str()) {
            if album.len() > 0 {
                data.album = Some(album.to_string());
            }
        }
        if let Some(art) = raw.get("mpris:artUrl").and_then(|a| a.as_str()) {
            if art.len() > 0 {
                data.art = Some(art.to_string());
            }
        }

        let (artist, featured) = Self::parse_artists(raw);
        data.artist = artist;
        data.featured = featured;

        data
    }

    fn parse_artists(raw : &HashMap<String, arg::Variant<Box<arg::RefArg>>>)
                     -> (Option<String>, Option<Vec<String>>) {
        let mut artist = None;
        let mut featured : Option<Vec<String>> = None;

        {
            let mut parse_iter = |it : Box<Iterator<Item = &arg::RefArg>>| {
                for a in it {
                    if let Some(a) = a.as_str() {
                        if a.len() > 0 {
                            if let Some(ref artist) = artist {
                                if artist != a {
                                    if let Some(ref mut featured) = featured {
                                        let a = a.to_string();
                                        if !featured.contains(&a) {
                                            featured.push(a);
                                        }
                                    } else {
                                        featured = Some(vec![a.to_string()]);
                                    }
                                }
                            } else {
                                artist = Some(a.to_string());
                            }
                        }
                    }
                }
            };

            if let Some(it) = raw.get("xesam:albumArtist").and_then(|a| a.0.as_iter()) {
                parse_iter(it);
            }
            if let Some(it) = raw.get("xesam:artist").and_then(|a| a.0.as_iter()) {
                parse_iter(it);
            }
        }

        (artist, featured)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use dbus::arg::{Variant, RefArg};

    fn make_variant<T: 'static + RefArg>(val: T) -> Variant<Box<RefArg>> {
        Variant(Box::new(val))
    }

    #[test]
    fn it_parses_simple_data() {
        let mut raw : HashMap<String, Variant<Box<RefArg>>> = HashMap::new();
        raw.insert("xesam:title".to_string(), make_variant("Brother".to_string()));
        raw.insert("xesam:artist".to_string(), make_variant(vec!["Murder By Death".to_string()]));
        raw.insert("mpris:artUrl".to_string(), make_variant("https://open.spotify.com/image/f568c1436c8a9063d21efdd901e8ce6fdc1029e3".to_string()));
        raw.insert("xesam:album".to_string(), make_variant("In Bocca Al Lupo".to_string()));
        raw.insert("mpris:length".to_string(), make_variant(230853000));
        raw.insert("xesam:url".to_string(), make_variant("https://open.spotify.com/track/7tFAnpi9kCBSiNkA6ZPSiZ".to_string()));
        raw.insert("xesam:albumArtist".to_string(), make_variant(vec!["Murder By Death".to_string()]));
        raw.insert("xesam:autoRating".to_string(), make_variant(0.25));
        raw.insert("mpris:trackid".to_string(), make_variant("spotify:track:7tFAnpi9kCBSiNkA6ZPSiZ".to_string()));
        raw.insert("xesam:discNumber".to_string(), make_variant(1));
        raw.insert("xesam:trackNumber".to_string(), make_variant(4));

        let metadata = MPRIS::parse_metadata(&raw);
        assert_eq!(Metadata {
            title: Some("Brother".to_string()),
            album: Some("In Bocca Al Lupo".to_string()),
            artist: Some("Murder By Death".to_string()),
            featured: None,
            art: Some("https://open.spotify.com/image/f568c1436c8a9063d21efdd901e8ce6fdc1029e3".to_string()),
        }, metadata);
    }

    #[test]
    fn it_parses_with_multiple_artists() {
        let mut raw : HashMap<String, Variant<Box<RefArg>>> = HashMap::new();
        raw.insert("xesam:url".to_string(), make_variant("https://open.spotify.com/track/5IJ7ltnKTfKowtCrVmhN7s".to_string()));
        raw.insert("xesam:discNumber".to_string(), make_variant(1));
        raw.insert("mpris:artUrl".to_string(), make_variant("https://open.spotify.com/image/7f201a3182356eb97966df061ffc2f38bbe83732".to_string()));
        raw.insert("mpris:length".to_string(), make_variant(167933000));
        raw.insert("xesam:albumArtist".to_string(), make_variant(vec![
            "David Orlowsky Trio".to_string(),
            "David Orlowsky".to_string(),
        ]));
        raw.insert("xesam:autoRating".to_string(), make_variant(0.08));
        raw.insert("xesam:trackNumber".to_string(), make_variant(6));
        raw.insert("mpris:trackid".to_string(), make_variant("spotify:track:5IJ7ltnKTfKowtCrVmhN7s".to_string()));
        raw.insert("xesam:album".to_string(), make_variant("Klezmer Kings".to_string()));
        raw.insert("xesam:title".to_string(), make_variant("Yossl Yossl".to_string()));
        raw.insert("xesam:artist".to_string(), make_variant(vec![
            "SAMUEL STEINBERG".to_string(),
            "Nellie Casman".to_string(),
        ]));

        let metadata = MPRIS::parse_metadata(&raw);
        assert_eq!(Metadata {
            title: Some("Yossl Yossl".to_string()),
            album: Some("Klezmer Kings".to_string()),
            artist: Some("David Orlowsky Trio".to_string()),
            featured: Some(vec![
                "David Orlowsky".to_string(), "SAMUEL STEINBERG".to_string(),
                "Nellie Casman".to_string()
            ]),
            art: Some("https://open.spotify.com/image/7f201a3182356eb97966df061ffc2f38bbe83732".to_string()),
        }, metadata);
    }
}
