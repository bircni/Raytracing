use glium::{
    glutin::{
        dpi::PhysicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    Surface,
};

pub fn main() {
    let model = obj::Obj::load("res/test.obj").unwrap();
    println!(
        "objects: {:?}",
        model.data.objects[0].groups[0].polys[0].0[0]
    );

    let window_builder = WindowBuilder::new()
    .with_title("Hello, world!")
    .with_resizable(true)
    .with_min_inner_size(PhysicalSize::new(800, 600));
    let context_builder = ContextBuilder::new();
    let event_loop = EventLoop::new();

    let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();

    event_loop.run(move |e, _, c| match e {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *c = ControlFlow::Exit;
        }
        Event::WindowEvent {..} => {}
        Event::RedrawRequested(_) => {
            let mut frame = display.draw();

            frame.clear_color(0.0, 0.0, 1.0, 1.0);

            frame.finish().unwrap();
        }
        _ => {}
    });
}
