use crate::{raytracer::Skybox, Color};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub max_bounces: u32,
    pub samples: u32,
    pub background_color: Color,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub skybox: Option<Skybox>,
}

mod yaml {
    use super::Settings;
    use crate::{raytracer::Skybox, Color};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SettingsDef {
        pub max_bounces: u32,
        pub samples: u32,
        #[serde(with = "super::super::yaml::color")]
        pub background_color: Color,
        #[serde(with = "super::super::yaml::color")]
        pub ambient_color: Color,
        pub skybox: Option<String>,
    }

    impl<'de> Deserialize<'de> for Settings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            SettingsDef::deserialize(deserializer).map(|yaml_extras| Settings {
                max_bounces: yaml_extras.max_bounces,
                samples: yaml_extras.samples,
                background_color: yaml_extras.background_color,
                ambient_color: yaml_extras
                    .ambient_color
                    .try_normalize(0.0)
                    .unwrap_or_default(),
                ambient_intensity: yaml_extras.ambient_color.norm(),
                skybox: Skybox::from_string(yaml_extras.skybox),
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
                background_color: self.background_color,
                ambient_color: self.ambient_color * self.ambient_intensity,
                skybox: Some(self.skybox.unwrap_or(Skybox::None).as_string()),
            }
            .serialize(serializer)
        }
    }
}
