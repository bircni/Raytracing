use super::Color;
use image::RgbImage;
use std::path::{Path, PathBuf};

#[derive(PartialEq, Debug, Clone)]
pub enum Skybox {
    Image { path: PathBuf, image: RgbImage },
    Color(Color),
}

impl Default for Skybox {
    fn default() -> Self {
        Self::Color(Color::new(0.16, 0.16, 0.16))
    }
}

mod yaml {
    use super::Skybox;
    use crate::scene::Color;
    use serde::{de::Error, Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum SkyboxDef {
        Path(String),
        Color(Color),
    }

    impl<'de> Deserialize<'de> for Skybox {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            SkyboxDef::deserialize(deserializer).and_then(|yaml_extras| match yaml_extras {
                SkyboxDef::Path(path) => Self::load_from_path(path)
                    .map_err(|e| Error::custom(format!("Failed to load skybox: {e}"))),
                SkyboxDef::Color(color) => Ok(Self::Color(color)),
            })
        }
    }

    impl Serialize for Skybox {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                Self::Image { path, .. } => SkyboxDef::Path(path.to_string_lossy().to_string()),
                Self::Color(color) => SkyboxDef::Color(*color),
            }
            .serialize(serializer)
        }
    }
}

impl Skybox {
    fn load_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let image = image::open(path.as_ref())?.into_rgb8();

        Ok(Self::Image {
            path: path.as_ref().to_path_buf(),
            image,
        })
    }
}
