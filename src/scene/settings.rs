
use serde::{Deserialize, Serialize};

use crate::Color;
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Settings {
    pub max_bounces: u32,
    pub samples: u32,
    pub background_color: Color,
}


impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        mod yaml {
            use nalgebra::Point3;
            use serde::Deserialize;

            use crate::Color;

            #[derive(Deserialize)]
            pub struct Settings {
                pub max_bounces: u32,
                pub samples: u32,
                #[serde(with = "super::super::yaml::color_rgb")]
                pub background_color: Color,
            }
        }

        yaml::Settings::deserialize(deserializer).map(|yaml_extras| Settings {
            max_bounces: yaml_extras.max_bounces,
            samples: yaml_extras.samples,
            background_color: yaml_extras.background_color,
        })
    }
}