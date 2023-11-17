use anyhow::Context;
use serde::Deserialize;
use serde_yaml::Value;

use self::{camera::Camera, light::Light, object::Object, settings::Settings};

mod camera;
pub mod light;
pub mod object;
pub mod settings;
pub mod triangle;

#[derive(Debug, Clone)]
pub struct Scene {
    pub objects: Vec<Object>,
    pub lights: Vec<Light>,
    pub camera: Camera,
    pub settings: Settings,
}

impl<'de> Deserialize<'de> for Scene {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = Value::deserialize(deserializer)?;

        let objects = config
            .get("models")
            .ok_or(serde::de::Error::missing_field("models"))?
            .as_sequence()
            .ok_or(serde::de::Error::custom("models is not a sequence"))?
            .iter()
            .map(|s| serde_yaml::from_value::<Object>(s.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        let lights = config
            .get("point_lights")
            .ok_or(serde::de::Error::missing_field("point_lights"))?
            .as_sequence()
            .ok_or(serde::de::Error::custom("point_lights is not a sequence"))?
            .iter()
            .map(|s| serde_yaml::from_value::<Light>(s.clone()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(serde::de::Error::custom)?;

        let camera = serde_yaml::from_value::<Camera>(
            config
                .get("camera")
                .ok_or(serde::de::Error::missing_field("camera"))?
                .clone(),
        )
        .map_err(serde::de::Error::custom)?;

        let settings = serde_yaml::from_value::<Settings>(
            config
                .get("extra_args")
                .ok_or(serde::de::Error::missing_field("extra_args"))?
                .clone(),
        )
        .map_err(serde::de::Error::custom)?;

        Ok(Scene {
            objects,
            lights,
            camera,
            settings,
        })
    }
}

mod yaml {
    pub mod point3_xyz {
        use nalgebra::Point3;
        use serde::Deserialize;

        #[derive(serde::Deserialize)]
        pub struct PointXYZ {
            x: f32,
            y: f32,
            z: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Point3<f32>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let point = PointXYZ::deserialize(deserializer)?;
            Ok(Point3::new(point.x, point.y, point.z))
        }
    }

    pub mod vector3_xyz {
        use nalgebra::Vector3;
        use serde::Deserialize;

        #[derive(serde::Deserialize)]
        pub struct VectorXYZ {
            x: f32,
            y: f32,
            z: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vector3<f32>, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let point = VectorXYZ::deserialize(deserializer)?;
            Ok(Vector3::new(point.x, point.y, point.z))
        }
    }

    pub mod color_rgb {
        use serde::Deserialize;

        use crate::Color;

        #[derive(serde::Deserialize)]
        pub struct ColorRGB {
            r: f32,
            g: f32,
            b: f32,
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let color = ColorRGB::deserialize(deserializer)?;
            Ok(Color::new(color.r, color.g, color.b))
        }
    }
}

impl Scene {
    pub fn load<P: AsRef<std::path::Path>>(path: P) -> anyhow::Result<Scene> {
        let s = std::fs::read_to_string(path.as_ref()).context(format!(
            "Failed to read file from path: {}",
            path.as_ref().display()
        ))?;

        serde_yaml::from_str::<Scene>(s.as_str()).context("Failed to parse yaml config file")
    }
}

#[test]
fn test_load_scene() -> anyhow::Result<()> {
    let scene = Scene::load("./res/config.yaml").context("Failed to load scene")?;

    assert_eq!(scene.objects.len(), 1);
    assert_eq!(scene.lights.len(), 2);

    Ok(())
}
