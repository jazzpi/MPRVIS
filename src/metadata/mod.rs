extern crate dbus;

use self::dbus::arg;
use self::dbus::arg::RefArg;
use self::dbus::stdintf::org_freedesktop_dbus::Properties;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct Metadata {
    title: Option<String>,
    artist: Option<String>,
    featured: Option<Vec<String>>,
    art: Option<String>,
}

pub fn get_current(conn : &dbus::Connection) -> Metadata {
    let player = conn.with_path("org.mpris.MediaPlayer2.spotify",
                                "/org/mpris/MediaPlayer2", 500);
    let metadata = player.get("org.mpris.MediaPlayer2.Player", "Metadata")
                         .unwrap();
    
    parse_metadata(&metadata)
}

fn parse_metadata(raw : &HashMap<String, arg::Variant<Box<arg::RefArg>>>) -> Metadata {
    let mut data = Metadata {
        title: None,
        artist: None,
        featured: None,
        art: None,
    };
    
    if let Some(title) = raw.get("xesam:title").and_then(|t| t.as_str()) {
        if title.len() > 0 {
            data.title = Some(title.to_string());
        }
    }
    if let Some(art) = raw.get("mpris:artUrl").and_then(|a| a.as_str()) {
        if art.len() > 0 {
            data.art = Some(art.to_string());
        }
    }
    
    let (artist, featured) = parse_artists(raw);
    data.artist = artist;
    data.featured = featured;
    
    data
}

fn parse_artists(raw : &HashMap<String, arg::Variant<Box<arg::RefArg>>>) -> (Option<String>, Option<Vec<String>>) {
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
        
        let metadata = parse_metadata(&raw);
        assert_eq!(Metadata {
            title: Some("Brother".to_string()),
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

        let metadata = parse_metadata(&raw);
        assert_eq!(Metadata {
            title: Some("Yossl Yossl".to_string()),
            artist: Some("David Orlowsky Trio".to_string()),
            featured: Some(vec![
                "David Orlowsky".to_string(), "SAMUEL STEINBERG".to_string(),
                "Nellie Casman".to_string()
            ]),
            art: Some("https://open.spotify.com/image/7f201a3182356eb97966df061ffc2f38bbe83732".to_string()),
        }, metadata);
    }
}
