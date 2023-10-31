/*use std::borrow::Cow;


use anyhow::Context;
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

mod raytracer;
mod scene;

pub type Color = Vector3<f32>;

pub fn main() -> anyhow::Result<()> {
    std::fs::create_dir_all("logs").context("Failed to create logs directory")?;

    let scene = scene::Scene::load("./res/config.yaml")?;
    println!("{:?}", scene);

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
            File::create(format!(
                "logs/trayracer_{}.log",
                chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
            ))
            .context("Failed to create log file")?,
        ),
    ])
    .context("Failed to initialize logger")?;

    let window_builder = WindowBuilder::new()
        .with_title("TrayRacer!")
        .with_resizable(true)
        .with_inner_size(PhysicalSize::new(1200, 800));
    let context_builder = ContextBuilder::new();
    let event_loop = EventLoop::new();

    let display = glium::Display::new(window_builder, context_builder, &event_loop)
        .context("Failed to create display")?;

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
            .context("Failed to create texture")
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

            frame.finish().context("Failed to finish frame").unwrap();
        }
        Event::WindowEvent { .. } => {}
        Event::RedrawRequested(_) => {}
        _ => {}
    });
}*/

use std::borrow::Cow;
use eframe::egui;


fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200.0, 800.0)),
        ..Default::default()
    };
    eframe::run_native(
        "RayTracer!",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::<TrayRacerApp>::default()
        }),
    )
}

struct TrayRacerApp {
    sample_string: String,
    sample_num: u32,
}

impl Default for TrayRacerApp {
    fn default() -> Self {
        Self {
            sample_string: "Raytracing is fun!".to_owned(),
            sample_num: 42,
        }
    }
}

impl eframe::App for TrayRacerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            //horizontqal layout with multiple seperate vertical layouts to sit on top of displayed image
            ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.heading(format!("Global String variable now set to: {}", self.sample_string));
                ui.label(format!("Global Num variable now set to: {}", self.sample_num));
                ui.add(
                    //sample slider to modify a varaible on the fly
                    egui::Slider::new(&mut self.sample_num, 0..=100)
                );
            });
            ui.vertical(|ui|{
                 //radio menu to select a value to bind to variable sample_string
                 let test1:String = "test1".to_string();
                 let test2:String = "test2".to_string();
                 let test3:String = "test3".to_string();
                 ui.radio_value(&mut self.sample_string,  test1, "Option 1");
                 ui.radio_value(&mut self.sample_string,  test2, "Option 2");
                 ui.radio_value(&mut self.sample_string,  test3, "Option 3");
            });
            });

            //adding an image to the gui
            ui.image(egui::include_image!(
                "../res/Farbverlauf.jpg"
            ));
            
           
            
        });
    }
}