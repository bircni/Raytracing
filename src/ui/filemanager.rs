use std::path::PathBuf;

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

    pub fn check_file_extensions(files: Vec<egui::DroppedFile>) -> Vec<egui::DroppedFile> {
        files
            .into_iter()
            .filter(|file| {
                file.path
                    .as_ref()
                    .and_then(|ext| ext.extension())
                    .map(|path| path.to_string_lossy().to_lowercase())
                    .is_some_and(|ext| ext == "yaml" || ext == "yml" || ext == "obj")
            })
            .collect()
    }
}
