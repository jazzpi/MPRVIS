use mpris;

use std::sync::mpsc;
use std::thread;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use curl::easy::Easy;

mod fetcher;
use self::fetcher::FetcherExt;

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
    fetcher: fetcher::Fetcher,
}

impl Manager {
    pub fn new(done_tx: mpsc::Sender<mpris::Event>,
               request_rx: mpsc::Receiver<mpris::Metadata>) -> Self {
        let fetcher = fetcher::Fetcher::new();
        Manager {
            done_tx,
            request_rx,
            fetcher,
        }
    }

    pub fn run(&mut self) {
        for data in &self.request_rx {
            match self.fetcher.fetch(&data, self.done_tx.clone()) {
                Ok(_) => {
                    self.done_tx.send(mpris::Event::ArtDone(true));
                },
                Err(err) => {
                    eprintln!("Error while fetching art: {}", err);
                    self.done_tx.send(mpris::Event::ArtDone(false));
                }
            }
        }
    }
}
