use mpris;

use std::sync::mpsc;
use std::thread;
use std::env::args;
use std::error::Error;

use gtk;
use gtk::prelude::*;
use gio;
use gio::prelude::*;
use gdk;
use gdk::prelude::*;
use gdk_pixbuf;
use gdk_pixbuf::prelude::*;
use glib;
use cairo;

static mut GUI_INST: Option<GUI> = None;

/// Start the GUI
///
/// # Safety
///
/// This function may only be called once.
pub unsafe fn start(events_tx: mpsc::Sender<mpsc::Sender<mpris::Event>>) {
    let application = gtk::Application::new(
        "space.jazzpis.mprvis", gio::ApplicationFlags::empty()
    ).unwrap();

    application.connect_startup(move |app| build_ui(app, &events_tx));
    application.connect_activate(|app| {
        GUI_INST.as_ref().unwrap().raise_window(app);
    });

    application.run(&args().collect::<Vec<_>>());
}

unsafe fn build_ui(app: &gtk::Application,
                   events_tx: &mpsc::Sender<mpsc::Sender<mpris::Event>>) {
    GUI_INST = Some(GUI::new(app, events_tx));
}

pub struct GUI {
    window: gtk::ApplicationWindow,
    song_title: gtk::Label,
    artist: gtk::Label,
    album: gtk::Label,
    cover: gtk::DrawingArea,
    img: Option<gdk_pixbuf::Pixbuf>,
    playback_status: gtk::Label,
    events_tx: mpsc::Sender<mpsc::Sender<mpris::Event>>,
}

impl GUI {
    pub fn new(app: &gtk::Application,
               events_tx: &mpsc::Sender<mpsc::Sender<mpris::Event>>) -> Self {
        let builder = gtk::Builder::new_from_file(
            "/home/jasper/dev/mprvis/assets/gui.glade"
        );

        let window: gtk::ApplicationWindow = builder.get_object("window")
                                                    .unwrap();
        window.set_application(app);
        Self::setup_style(&window);

        let w = window.clone();
        window.connect_delete_event(move |_, _| {
            w.destroy();
            Inhibit(false)
        });

        let song_title = builder.get_object("song_title").unwrap();
        let artist = builder.get_object("artist").unwrap();
        let album = builder.get_object("album").unwrap();
        let playback_status = builder.get_object("playback_status").unwrap();

        let cover: gtk::DrawingArea = builder.get_object("cover").unwrap();
        cover.connect_draw(|_, context| {
            let gui = unsafe { GUI_INST.as_mut().unwrap() };
            gui.draw_cover(context)
        });

        window.show_all();

        let events_tx = events_tx.clone();

        let gui = Self {
            window,
            song_title,
            artist,
            album,
            playback_status,
            cover,
            img: None,
            events_tx,
        };
        gui.start_loop();
        gui
    }

    fn setup_style(win: &gtk::ApplicationWindow) {
        let provider = gtk::CssProvider::new();
        provider.connect_parsing_error(|_, section, error| {
            eprintln!("CSS parsing error in lines {}--{}: {:?}",
                      section.get_start_line(), section.get_end_line(),
                      error.description());
        });
        if provider.load_from_path(
            "/home/jasper/dev/mprvis/assets/gui.css"
        ).is_ok() {
            let style_context = win.get_style_context().unwrap();
            style_context.add_provider(&provider, 0);
            gtk::StyleContext::add_provider_for_screen(
                &gdk::Screen::get_default().unwrap(),
                &provider,
                0,
            );
        }
    }

    fn start_loop(&self) {
        let events_tx = self.events_tx.clone();
        thread::spawn(move || {
            let (tx, rx) = mpsc::channel();
            events_tx.send(tx);
            Self::run_loop(rx);
        });
    }

    fn run_loop(events_rx: mpsc::Receiver<mpris::Event>) {
        let mut art = Vec::<u8>::new();
        for mut ev in events_rx {
            match ev {
                mpris::Event::Data(ref metadata) => {
                    let metadata = metadata.clone();
                    glib::idle_add(move || {
                        unsafe {
                            GUI_INST.as_ref()
                                    .unwrap()
                                    .update_data(metadata.clone());
                        }
                        gtk::Continue(false)
                    });
                },
                mpris::Event::Playback(ref playback_status) => {
                    let playback_status = playback_status.clone();
                    glib::idle_add(move || {
                        unsafe {
                            GUI_INST.as_ref()
                                    .unwrap()
                                    .update_status(playback_status.clone());
                        }
                        gtk::Continue(false)
                    });
                },
                mpris::Event::ArtData(ref mut data) => {
                    art.append(data);
                },
                mpris::Event::ArtDone(success) => {
                    if success {
                        let art = art.clone();
                        glib::idle_add(move || {
                            unsafe {
                                GUI_INST.as_mut()
                                        .unwrap()
                                        .update_art(&art);
                            }
                            gtk::Continue(false)
                        });
                    }
                    art = Vec::new();
                }
            }
        }
    }

    pub fn update_data(&self, data: mpris::Metadata) {
        self.song_title.set_text(
            &data.title.unwrap_or("No song playing!".to_string())
        );
        self.artist.set_text(
            &data.artist.unwrap_or("".to_string())
        );
        self.album.set_text(
            &data.album.unwrap_or("".to_string())
        );
    }

    pub fn update_status(&self, playback_status: mpris::PlaybackStatus) {
        let status = match playback_status {
            mpris::PlaybackStatus::Paused => "Paused",
            mpris::PlaybackStatus::Playing => "Playing",
            mpris::PlaybackStatus::Stopped => "Stopped",
        };

        self.playback_status.set_text(status);
    }

    pub fn update_art(&mut self, data: &[u8]) {
        let loader = gdk_pixbuf::PixbufLoader::new();
        loader.write(data);
        loader.close();
        self.img = loader.get_pixbuf();
        if self.img.is_none() {
            eprintln!("Couldn't parse image!");
        }
        self.cover.queue_draw();
    }

    fn raise_window(&self, app: &gtk::Application) {
        self.window.get_window().unwrap().raise();
    }

    fn draw_cover(&self, context: &cairo::Context) -> Inhibit {
        let width = self.cover.get_allocated_width();
        let height = self.cover.get_allocated_height();
        let size = width.min(height);
        let (x, y) = ((width - size) as f64 / 2., (height - size) as f64 / 2.);

        context.set_source_rgb(0., 0., 0.);
        context.paint();
        if let Some(ref img) = self.img {
            let img = img.scale_simple(
                size, size, gdk_pixbuf::InterpType::Bilinear
            ).unwrap();
            let surf = cairo::Context::cairo_surface_create_from_pixbuf(
                &img, 0, self.window.get_window().as_ref()
            ).unwrap();
            context.set_source_surface(&surf, x, y);
        }
        context.paint();
        context.set_source_rgba(0., 0., 0., 0.5);
        context.paint();
        Inhibit(false)
    }
}
