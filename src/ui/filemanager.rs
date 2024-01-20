use std::path::PathBuf;

use egui::{Align2, Color32, Context, DroppedFile, Id, InputState, LayerId, Order, TextStyle};
use log::warn;
use nalgebra::{Scale3, Translation3, UnitQuaternion};

use crate::scene::{Object, Scene};

use super::yamlmenu::YamlMenu;

pub struct FileManager {}

impl FileManager {
    pub fn handle_file(path: &PathBuf, scene: &mut Option<Scene>) {
        if let Some(ext) = path
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase().clone())
        {
            match ext.as_str() {
                "yaml" | "yml" => YamlMenu::load_scene_from_path(scene, path),
                "obj" => {
                    if let Some(scene) = scene.as_mut() {
                        match Object::from_obj(
                            path,
                            Translation3::identity(),
                            UnitQuaternion::identity(),
                            Scale3::identity(),
                        ) {
                            Ok(object) => scene.objects.push(object),
                            Err(e) => warn!("Failed to load object: {}", e),
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn check_file_extensions(files: Vec<DroppedFile>) -> Vec<DroppedFile> {
        files
            .into_iter()
            .filter(|file| Self::check_extension(&file.path, &["yaml", "yml", "obj"]).0)
            .collect()
    }

    fn check_extension(path: &Option<PathBuf>, target_exts: &[&str]) -> (bool, Option<String>) {
        path.as_ref()
            .and_then(|ext| ext.extension())
            .map(|path| path.to_string_lossy().to_lowercase())
            .map_or((false, None), |ext| {
                (
                    target_exts
                        .iter()
                        .any(|&target| ext.eq_ignore_ascii_case(target)),
                    Some(ext),
                )
            })
    }

    pub fn hovered_file(ctx: &Context, scene: &Option<&mut Scene>) {
        if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
            let painter =
                ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));
            if let Some(hovered) = ctx.input(|i| i.raw.hovered_files.clone()).first() {
                let extension = Self::check_extension(&hovered.path, &["yaml", "yml", "obj"]);
                let screen_rect = ctx.input(InputState::screen_rect);
                painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
                painter.text(
                    screen_rect.center(),
                    Align2::CENTER_CENTER,
                    match scene {
                        Some(_scene) => {
                            if extension.1 == Some("obj".to_string()) {
                                "Drop the .obj file here to add it to the scene"
                            } else if extension.1 == Some("yaml".to_string())
                                || extension.1 == Some("yml".to_string())
                            {
                                "Drop the .yaml file here to load another scene"
                            } else {
                                "This file type is not supported"
                            }
                        }
                        None => "Drop a .yaml file here to load a new scene",
                    },
                    TextStyle::Heading.resolve(&ctx.style()),
                    Color32::WHITE,
                );
            }
        }
    }
}
