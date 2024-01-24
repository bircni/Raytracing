use super::{Color, Skybox};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    // TODO: Actually use these
    pub max_bounces: u32,
    pub samples: u32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub skybox: Skybox,
    pub anti_aliasing: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_bounces: 4,
            samples: 1,
            ambient_color: Color::new(0.34, 0.14, 0.04).normalize(),
            ambient_intensity: 0.2,
            skybox: Skybox::default(),
            anti_aliasing: false,
        }
    }
}

mod yaml {
    use crate::scene::{Color, Skybox};

    use super::Settings;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SettingsDef {
        pub max_bounces: u32,
        pub samples: u32,
        #[serde(with = "super::super::yaml::color")]
        pub ambient_color: Color,
        pub skybox: Skybox,
        pub anti_aliasing: bool,
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
                skybox: yaml_extras.skybox,
                anti_aliasing: yaml_extras.anti_aliasing,
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
                anti_aliasing: self.anti_aliasing,
            }
            .serialize(serializer)
        }
    }
}
