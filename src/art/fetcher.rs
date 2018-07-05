use mpris;

use std::sync::mpsc;
use std::collections::HashMap;

use curl::easy::Easy;

pub trait FetcherExt {
    fn fetch(&mut self, data: &mpris::Metadata, tx: mpsc::Sender<mpris::Event>)
             -> Result<(), String>;
}

pub struct Fetcher {
    web: WebFetcher,
    spotify: SpotifyFetcher,
}

impl Fetcher {
    pub fn new() -> Self {
        let web = WebFetcher {
            cache: HashMap::new(),
        };
        let spotify = SpotifyFetcher {
            cache: HashMap::new(),
        };
        Fetcher {
            web,
            spotify,
        }
    }
}

impl FetcherExt for Fetcher {
    fn fetch(&mut self, data: &mpris::Metadata, tx: mpsc::Sender<mpris::Event>)
             -> Result<(), String> {
        if let Some(ref url) = data.art {
            if url.starts_with("https://open.spotify.com/image") {
                self.spotify.fetch(data, tx)
            } else if url.starts_with("http://") ||
                      url.starts_with("https://") {
                self.web.fetch(data, tx)
            } else {
                Err(format!("Unkown URL scheme: {}", url))
            }
        } else {
            Err("No Art URL set!".to_string())
        }
    }
}

trait FetcherWithCache {
    type Key;

    fn get_key(&self, data: &mpris::Metadata) -> Result<Self::Key, String>;
    fn cache_get(&self, key: &Self::Key) -> Option<Vec<u8>>;
    fn cache_set(&mut self, key: &Self::Key, data: Vec<u8>);
    fn fetch_uncached(&self, key: &Self::Key, tx: mpsc::Sender<mpris::Event>)
                      -> Result<Vec<u8>, String>;
}

impl<T: FetcherWithCache> FetcherExt for T {
    fn fetch(&mut self, data: &mpris::Metadata, tx: mpsc::Sender<mpris::Event>)
             -> Result<(), String> {
        let key = self.get_key(data)?;
        if let Some(data) = self.cache_get(&key) {
            tx.send(mpris::Event::ArtData(data));
            Ok(())
        } else {
            self.fetch_uncached(&key, tx).map(|data| {
                self.cache_set(&key, data);
            })
        }
    }
}

trait WebFetcherExt {
    fn get_url(&self, data: &mpris::Metadata) -> Result<String, String>;
    fn cache_get(&self, key: &String) -> Option<Vec<u8>>;
    fn cache_set(&mut self, key: &String, data: Vec<u8>);
}

impl<T: WebFetcherExt> FetcherWithCache for T {
    type Key = String;

    fn get_key(&self, data: &mpris::Metadata) -> Result<String, String> {
        self.get_url(data)
    }

    fn cache_get(&self, key: &String) -> Option<Vec<u8>> {
        WebFetcherExt::cache_get(self, key)
    }

    fn cache_set(&mut self, key: &String, data: Vec<u8>) {
        WebFetcherExt::cache_set(self, key, data)
    }

    fn fetch_uncached(&self, url: &String, tx: mpsc::Sender<mpris::Event>)
                      -> Result<Vec<u8>, String> {
        println!("Fetching {:?}", url);
        let mut handle = Easy::new();
        handle.url(&url).unwrap();
        handle.get(true).unwrap();
        handle.follow_location(true).unwrap();
        let mut status = None;
        let mut res = Vec::new();
        {
            let mut transfer = handle.transfer();
            transfer.header_function(|new_data| {
                // new_data is always one header line
                if let Ok(s) = String::from_utf8(new_data.to_vec()) {
                    if s.starts_with("HTTP/1.1") {
                        status = Some(s[9..].trim().to_string());
                    }
                }
                true
            }).unwrap();
            transfer.write_function(|new_data| {
                res.extend_from_slice(new_data);
                tx.send(mpris::Event::ArtData(new_data.to_vec()));
                Ok(new_data.len())
            }).unwrap();
            transfer.perform().unwrap();
        }
        if status == Some("200 OK".to_string()) {
            Ok(res)
        } else {
            Err(format!("Download Error: {:?}", status))
        }
    }
}

struct WebFetcher {
    cache: HashMap<String, Vec<u8>>,
}

impl WebFetcherExt for WebFetcher {
    fn get_url(&self, data: &mpris::Metadata) -> Result<String, String> {
        data.art
            .clone()
            .ok_or("No Art URL set!".to_string())
    }

    fn cache_get(&self, key: &String) -> Option<Vec<u8>> {
        self.cache.get(key).map(|d| d.clone())
    }

    fn cache_set(&mut self, key: &String, data: Vec<u8>) {
        self.cache.insert(key.clone(), data);
    }
}

struct SpotifyFetcher {
    cache: HashMap<String, Vec<u8>>,
}

impl WebFetcherExt for SpotifyFetcher {
    fn get_url(&self, data: &mpris::Metadata) -> Result<String, String> {
        data.art
            .clone()
            .map(|s| s.replacen("open.spotify.com", "i.scdn.co", 1))
            .ok_or("No Art URL set!".to_string())
    }

    fn cache_get(&self, key: &String) -> Option<Vec<u8>> {
        self.cache.get(key).map(|d| d.clone())
    }

    fn cache_set(&mut self, key: &String, data: Vec<u8>) {
        self.cache.insert(key.clone(), data);
    }
}
