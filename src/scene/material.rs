use image::RgbImage;

#[derive(Debug, Clone)]
pub struct Material {
    pub name: String,
    pub ka: Option<[f32; 3]>,
    pub kd: Option<[f32; 3]>,
    pub ks: Option<[f32; 3]>,
    pub ke: Option<[f32; 3]>,
    pub km: Option<f32>,
    pub tf: Option<[f32; 3]>,
    pub ns: Option<f32>,
    pub ni: Option<f32>,
    pub tr: Option<f32>,
    pub d: Option<f32>,
    pub illum: Option<i32>,
    pub map_ka: Option<RgbImage>,
    pub map_kd: Option<RgbImage>,
    pub map_ks: Option<RgbImage>,
    pub map_ke: Option<RgbImage>,
    pub map_ns: Option<RgbImage>,
    pub map_d: Option<RgbImage>,
    pub map_bump: Option<RgbImage>,
    pub map_refl: Option<RgbImage>,
}
