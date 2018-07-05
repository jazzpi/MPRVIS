use mpris;

use std::sync::mpsc;
use std::thread;

use curl::easy::Easy;

pub fn start(done_tx: mpsc::Sender<mpris::Event>)
             -> mpsc::Sender<mpris::Metadata> {
    let (tmp_tx, tmp_rx) = mpsc::channel();

    thread::spawn(move || {
        let (req_tx, req_rx) = mpsc::channel();
        tmp_tx.send(req_tx).unwrap();

        let mut manager = Manager::new(done_tx, req_rx);
        manager.run();
    });

    tmp_rx.recv().unwrap()
}

struct Manager {
    done_tx: mpsc::Sender<mpris::Event>,
    request_rx: mpsc::Receiver<mpris::Metadata>,
}

impl Manager {
    pub fn new(done_tx: mpsc::Sender<mpris::Event>,
               request_rx: mpsc::Receiver<mpris::Metadata>) -> Self {
        Manager {
            done_tx,
            request_rx,
        }
    }

    pub fn run(&mut self) {
        for data in &self.request_rx {
            self.fetch_art(&data);
        }
    }

    fn fetch_art(&self, data: &mpris::Metadata) {
        if let Some(ref url) = data.art {
            if url.starts_with("https://open.spotify.com/image") {
                self.fetch_spot_art(url);
            } else if url.starts_with("http://") ||
                      url.starts_with("https://") {
                self.fetch_web_art(url);
            } else {
                eprintln!("Unknown artUrl scheme: {}", url);
            }
        }
    }

    fn fetch_spot_art(&self, url: &str) {
        self.fetch_web_art(&url.replacen("open.spotify.com", "i.scdn.co", 1));
    }

    fn fetch_web_art(&self, url: &str) {
        let url = url.to_string();
        let tx = self.done_tx.clone();
        thread::spawn(move || {
            let mut handle = Easy::new();
            handle.url(&url).unwrap();
            handle.get(true).unwrap();
            handle.follow_location(true).unwrap();
            let mut status = None;
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
                    tx.send(mpris::Event::ArtData(new_data.to_vec()));
                    Ok(new_data.len())
                }).unwrap();
                transfer.perform().unwrap();
            }
            if status == Some("200 OK".to_string()) {
                tx.send(mpris::Event::ArtDone(true));
            } else {
                eprintln!("Error while fetching art: {:?}", status);
                return
            }
        });
    }
}
