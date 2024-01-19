/// provide (de)serialization for Color (aka. Vector3<f32>) from r,g,b fields
pub mod color {
    use serde::{Deserialize, Serialize};

    use crate::scene::Color;

    #[derive(Serialize, Deserialize)]
    struct Yaml {
        r: f32,
        g: f32,
        b: f32,
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let color = Yaml::deserialize(deserializer)?;
        Ok(Color::new(color.r, color.g, color.b))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Yaml {
            r: color.x,
            g: color.y,
            b: color.z,
        }
        .serialize(serializer)
    }
}

/// provide (de)serialization for Point3<f32> from x,y,z fields
pub mod point {
    use nalgebra::Point3;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Yaml {
        x: f32,
        y: f32,
        z: f32,
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Point3<f32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let point = Yaml::deserialize(deserializer)?;
        Ok(Point3::new(point.x, point.y, point.z))
    }

    pub fn serialize<S>(point: &Point3<f32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Yaml {
            x: point.x,
            y: point.y,
            z: point.z,
        }
        .serialize(serializer)
    }
}

/// provide (de)serialization for Vector3<f32> from x,y,z fields
pub mod vector {
    use nalgebra::Vector3;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct Yaml {
        x: f32,
        y: f32,
        z: f32,
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vector3<f32>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let point = Yaml::deserialize(deserializer)?;
        Ok(Vector3::new(point.x, point.y, point.z))
    }

    pub fn serialize<S>(point: &Vector3<f32>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Yaml {
            x: point.x,
            y: point.y,
            z: point.z,
        }
        .serialize(serializer)
    }
}
