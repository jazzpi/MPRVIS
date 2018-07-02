use std::sync::Arc;
use std::cell::RefCell;

use conrod::{self, widget, Widget, Positionable, Colorable};
use conrod::backend::glium::glium::{self, Surface};

widget_ids!(struct Ids { img, text });

pub struct GUI {
    events_loop: RefCell<glium::glutin::EventsLoop>,
    display: glium::Display,
    ui: RefCell<conrod::Ui>,
    ids: Ids,
    renderer: RefCell<conrod::backend::glium::Renderer>,
    image_map: conrod::image::Map<glium::texture::Texture2d>,
    events: Arc<RefCell<Vec<glium::glutin::Event>>>,
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
        
        let events = Arc::new(RefCell::new(vec![]));
        
        GUI {
            events_loop,
            display,
            ui,
            ids,
            renderer,
            image_map,
            events,
        }
    }
    
    pub fn run_loop(&self) {
        'render: loop {
            let mut events = self.events.borrow_mut();
            events.clear();
            
            // Get new events since the last frame
            let mut events_loop = self.events_loop.borrow_mut();
            events_loop.poll_events(|event| events.push(event));
            
            // Wait for one event
            if events.is_empty() {
                events_loop.run_forever(|event| {
                    events.push(event);
                    glium::glutin::ControlFlow::Break
                });
            }
            
            let mut ui = self.ui.borrow_mut();
            
            // Process events
            for event in events.drain(..) {
                match event.clone() {
                    glium::glutin::Event::WindowEvent {event, ..} => {
                        match event {
                            glium::glutin::WindowEvent::Closed |
                            glium::glutin::WindowEvent::KeyboardInput {
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
                
                let input = match conrod::backend::winit::convert_event(event, &self.display) {
                    None => continue,
                    Some(input) => input,
                };
                ui.handle_event(input);
            }
            
            let ui = &mut ui.set_widgets();
            
            widget::Text::new("Hello World!")
                .middle_of(ui.window)
                .color(conrod::color::WHITE)
                .font_size(32)
                .set(self.ids.text, ui);
            
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
