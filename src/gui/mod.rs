use mpris;

use std::cell::RefCell;
use std::sync::mpsc;
use std::thread;

use conrod::{self, widget, Widget, Positionable, Colorable, Sizeable};
use conrod::position::Dimension;
use conrod::backend::glium::glium::{self, Surface};

enum Event {
    MPRIS(mpris::Event),
    Backend(glium::glutin::Event),
}

struct EventCollector {
    mpris_receiver: Option<thread::JoinHandle<()>>,
    loop_proxy: glium::glutin::EventsLoopProxy,
    events_tx: mpsc::Sender<Event>,
}

impl<'a> EventCollector {
    pub fn new(loop_proxy: glium::glutin::EventsLoopProxy,
               events_tx: mpsc::Sender<Event>) -> Self {
        Self {
            mpris_receiver: None,
            loop_proxy,
            events_tx,
        }
    }

    pub fn start(&mut self) {
        // Ugly hack since we need our mpris rx in the thread and we can't move
        // receivers between threads
        let (tmp_tx, tmp_rx) = mpsc::channel();
        let events_tx = self.events_tx.clone();
        let loop_proxy = self.loop_proxy.clone();

        self.mpris_receiver = Some(thread::spawn(move || {
            let (mpris_tx, mpris_rx) = mpsc::channel();
            tmp_tx.send(mpris_tx)
                  .expect("Couldn't send MPRIS TX to EventCollector::start");

            while let Ok(ev) = mpris_rx.recv() {
                println!("MPRIS Event: {:?}", ev);
                if events_tx.send(Event::MPRIS(ev)).is_ok() {
                    if loop_proxy.wakeup().is_err() {
                        // Events loop is gone
                        break;
                    }
                } else {
                    // Events RX is gone
                    break;
                }
            }
        }));

        mpris::MPRIS::start(tmp_rx.recv().expect(
            "Couldn't receive MPRIS TX from MPRIS RX thread"));
    }
}

widget_ids!(struct Ids { img, song_title, song_artist, song_album, status });

pub struct GUI {
    events_loop: RefCell<glium::glutin::EventsLoop>,
    display: glium::Display,
    ui: RefCell<conrod::Ui>,
    ids: Ids,
    renderer: RefCell<conrod::backend::glium::Renderer>,
    image_map: conrod::image::Map<glium::texture::Texture2d>,
    event_collector: EventCollector,
    events_rx: mpsc::Receiver<Event>,
    events: RefCell<Vec<Event>>,
}

impl GUI {
    pub fn new(width: u32, height: u32) -> Self {
        let events_loop = RefCell::new(glium::glutin::EventsLoop::new());
        let window = glium::glutin::WindowBuilder::new()
            .with_title("MPRVis")
            .with_dimensions(width, height);
        let context = glium::glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_multisampling(4);
        let display = glium::Display::new(window, context, &events_loop.borrow()).unwrap();

        let ui = RefCell::new(conrod::UiBuilder::new([width as f64, height as f64]).build());

        let ids = Ids::new(ui.borrow_mut().widget_id_generator());

        // TODO: Add font
        ui.borrow_mut().fonts.insert_from_file("/home/jasper/.cargo/registry/src/github.com-1ecc6299db9ec823/conrod-0.60.0/assets/fonts/NotoSans/NotoSans-Regular.ttf").unwrap();

        let renderer = RefCell::new(conrod::backend::glium::Renderer::new(&display).unwrap());

        let image_map = conrod::image::Map::<glium::texture::Texture2d>::new();

        let (events_tx, events_rx) = mpsc::channel();

        let event_collector = EventCollector::new(
            events_loop.borrow().create_proxy(), events_tx);

        let events = RefCell::new(vec![]);

        GUI {
            events_loop,
            display,
            ui,
            ids,
            renderer,
            image_map,
            event_collector,
            events_rx,
            events,
        }
    }

    pub fn run_loop(&mut self) {
        self.event_collector.start();

        let mut title = "No song playing!".to_string();
        let mut artist = "".to_string();
        let mut album = "".to_string();
        let mut status = "Stopped".to_string();

        'render: loop {
            let mut events = self.events.borrow_mut();
            events.clear();

            // Get new events since the last frame
            let mut events_loop = self.events_loop.borrow_mut();
            events_loop.poll_events(|event| events.push(Event::Backend(event)));

            loop {
                let res = self.events_rx.try_recv();
                match res {
                    Ok(ev) => events.push(ev),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => break 'render,
                }
            }

            // Wait for one event
            if events.is_empty() {
                events_loop.run_forever(|event| {
                    events.push(Event::Backend(event));
                    glium::glutin::ControlFlow::Break
                });
            }

            let mut ui = self.ui.borrow_mut();

            // Process events
            for event in events.drain(..) {
                match event {
                    Event::MPRIS(ev) => {
                        match ev {
                            mpris::Event::Data(metadata) => {
                                title = metadata.title.unwrap_or(
                                    "No song playing!".to_string());
                                artist = metadata.artist.unwrap_or(
                                    "".to_string());
                                album = metadata.album.unwrap_or(
                                    "".to_string());
                            },
                            mpris::Event::Playback(playback_status) => {
                                match playback_status {
                                    mpris::PlaybackStatus::Paused =>
                                        status = "Paused".to_string(),
                                    mpris::PlaybackStatus::Playing =>
                                        status = "Playing".to_string(),
                                    mpris::PlaybackStatus::Stopped =>
                                        status = "Stopped".to_string(),
                                }
                            }
                        }
                    },
                    Event::Backend(ev) => {
                        match ev {
                            glium::glutin::Event::WindowEvent {ref event, ..} => {
                                match event {
                                    &glium::glutin::WindowEvent::Closed |
                                    &glium::glutin::WindowEvent::KeyboardInput {
                                        input : glium::glutin::KeyboardInput {
                                            virtual_keycode: Some(glium::glutin::VirtualKeyCode::Escape),
                                            ..
                                        },
                                        ..
                                    } => break 'render,
                                    _ => (),
                                }
                            },
                            _ => (),
                        }

                        let input = match conrod::backend::winit::convert_event(ev, &self.display) {
                            None => continue,
                            Some(input) => input,
                        };
                        ui.handle_event(input);
                    },
                }
            }

            let ui = &mut ui.set_widgets();

            let window_width = Dimension::Absolute(ui.w_of(ui.window).unwrap());

            widget::Text::new(title.as_str())
                .middle_of(ui.window)
                .x_dimension(window_width)
                .center_justify()
                .color(conrod::color::WHITE)
                .font_size(32)
                .set(self.ids.song_title, ui);
            widget::Text::new(artist.as_str())
                .middle_of(ui.window)
                .x_dimension(window_width)
                .center_justify()
                .down_from(self.ids.song_title, 0.0)
                .color(conrod::color::DARK_GRAY)
                .font_size(24)
                .set(self.ids.song_artist, ui);
            widget::Text::new(album.as_str())
                .middle_of(ui.window)
                .x_dimension(window_width)
                .center_justify()
                .down_from(self.ids.song_artist, 0.0)
                .color(conrod::color::DARK_GRAY)
                .font_size(24)
                .set(self.ids.song_album, ui);

            widget::Text::new(status.as_str())
                .mid_bottom_with_margin_on(ui.window, 10.0)
                .x_dimension(window_width)
                .center_justify()
                .color(conrod::color::DARK_GRAY)
                .font_size(24)
                .set(self.ids.status, ui);

            if let Some(primitives) = ui.draw_if_changed() {
                let mut renderer = self.renderer.borrow_mut();
                renderer.fill(&self.display, primitives, &self.image_map);
                let mut target = self.display.draw();
                target.clear_color(0., 0., 0., 1.);
                renderer.draw(&self.display, &mut target, &self.image_map).unwrap();
                target.finish().unwrap();
            }
        }
    }
}
