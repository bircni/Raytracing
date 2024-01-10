<<<<<<< HEAD
use crate::{raytracer::Skybox, Color};
=======
use crate::Color;

use super::Skybox;
>>>>>>> main

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub max_bounces: u32,
    pub samples: u32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
<<<<<<< HEAD
    pub skybox: Option<Skybox>,
=======
    pub skybox: Skybox,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_bounces: 4,
            samples: 1,
            ambient_color: Color::default(),
            ambient_intensity: 0.5,
            skybox: Skybox::default(),
        }
    }
>>>>>>> main
}

mod yaml {
    use crate::{scene::Skybox, Color};

    use super::Settings;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SettingsDef {
        pub max_bounces: u32,
        pub samples: u32,
        #[serde(with = "super::super::yaml::color")]
        pub ambient_color: Color,
        pub skybox: Skybox,
    }

    impl<'de> Deserialize<'de> for Settings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            SettingsDef::deserialize(deserializer).map(|yaml_extras| Settings {
                max_bounces: yaml_extras.max_bounces,
                samples: yaml_extras.samples,
                ambient_color: yaml_extras
                    .ambient_color
                    .try_normalize(0.0)
                    .unwrap_or_default(),
                ambient_intensity: yaml_extras.ambient_color.norm(),
<<<<<<< HEAD
                skybox: None,
=======
                skybox: yaml_extras.skybox,
>>>>>>> main
            })
        }
    }

    impl Serialize for Settings {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            SettingsDef {
                max_bounces: self.max_bounces,
                samples: self.samples,
                ambient_color: self.ambient_color * self.ambient_intensity,
                skybox: self.skybox.clone(),
            }
            .serialize(serializer)
        }
    }
}
