use std::borrow::Cow;

use glium::{
    glutin::{
        dpi::PhysicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    texture::{ClientFormat, MipmapsOption, RawImage2d, UncompressedFloatFormat},
    uniforms::MagnifySamplerFilter,
    BlitTarget, Rect, Surface, Texture2d,
};
use nalgebra::Vector3;
use simplelog::*;
use std::fs::File;

mod scene;

pub type Color = Vector3<f32>;

pub fn main() {
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");
    let log_file_name = format!(
        "logs/trayracer_{}.log",
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    );

    let log_level = if cfg!(debug_assertions) {
        LevelFilter::Trace
    } else {
        LevelFilter::Info
    };
    CombinedLogger::init(vec![
        TermLogger::new(
            log_level,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            log_level,
            Config::default(),
            File::create(log_file_name).unwrap(),
        ),
    ])
    .expect("Failed to initialize logger");

    let window_builder = WindowBuilder::new()
        .with_title("TrayRacer!")
        .with_resizable(true)
        .with_inner_size(PhysicalSize::new(1200, 800));
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
        Event::WindowEvent {
            event: WindowEvent::Resized(PhysicalSize { width, height }),
            ..
        } => {
            let texture = Texture2d::with_format(
                &display,
                RawImage2d {
                    data: Cow::Owned(
                        (0..height)
                            .flat_map(|y| {
                                (0..width).flat_map(move |x| {
                                    [x as f32 / width as f32, y as f32 / height as f32, 0.5, 1.0]
                                })
                            })
                            .collect::<Vec<f32>>(),
                    ),
                    width,
                    height,
                    format: ClientFormat::F32F32F32F32,
                },
                UncompressedFloatFormat::F32F32F32F32,
                MipmapsOption::NoMipmap,
            )
            .unwrap();

            let mut frame = display.draw();
            texture.as_surface().blit_color(
                &Rect {
                    left: 0,
                    bottom: 0,
                    width: texture.width(),
                    height: texture.height(),
                },
                &mut frame,
                &BlitTarget {
                    left: 0,
                    bottom: 0,
                    width: width as i32,
                    height: height as i32,
                },
                MagnifySamplerFilter::Linear,
            );

            frame.finish().unwrap();
        }
        Event::WindowEvent { .. } => {}
        Event::RedrawRequested(_) => {}
        _ => {}
    });
}
