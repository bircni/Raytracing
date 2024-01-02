use crate::Color;

use super::Skybox;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub max_bounces: u32,
    pub samples: u32,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub skybox: Skybox,
}

mod yaml {
    use crate::{scene::Skybox, Color};

    use super::Settings;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SettingsDef {
        pub max_bounces: Option<u32>,
        pub samples: Option<u32>,
        #[serde(with = "super::super::yaml::color")]
        pub ambient_color: Color,
        pub skybox: Option<Skybox>,
    }

    impl<'de> Deserialize<'de> for Settings {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            SettingsDef::deserialize(deserializer).map(|yaml_extras| Settings {
                max_bounces: yaml_extras.max_bounces.unwrap_or_default(),
                samples: yaml_extras.samples.unwrap_or_default(),
                ambient_color: yaml_extras
                    .ambient_color
                    .try_normalize(0.0)
                    .unwrap_or_default(),
                ambient_intensity: yaml_extras.ambient_color.norm(),
                skybox: yaml_extras.skybox.unwrap_or_default(),
            })
        }
    }

    impl Serialize for Settings {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            SettingsDef {
                max_bounces: Some(self.max_bounces),
                samples: Some(self.samples),
                ambient_color: self.ambient_color * self.ambient_intensity,
                skybox: Some(self.skybox.clone()),
            }
            .serialize(serializer)
        }
    }
}
