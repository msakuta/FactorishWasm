use super::{shader_bundle::ShaderBundle, utils::load_texture};
use cgmath::{Matrix4, Vector3};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{ImageBitmap, WebGlBuffer, WebGlRenderingContext as GL, WebGlTexture};

const FWIDTH: f64 = 100.;
const FHEIGHT: f64 = 100.;

pub(crate) struct Assets {
    pub world_transform: Matrix4<f64>,

    pub tex_dirt: WebGlTexture,
    pub tex_iron: WebGlTexture,
    pub tex_copper: WebGlTexture,
    pub tex_coal: WebGlTexture,
    pub tex_stone: WebGlTexture,
    pub tex_back: WebGlTexture,

    pub sprite_shader: Option<ShaderBundle>,
    pub textured_shader: Option<ShaderBundle>,

    pub rect_buffer: Option<WebGlBuffer>,
}

impl Assets {
    pub fn new(context: &GL, image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture_local = |path| -> Result<WebGlTexture, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                let ret = load_texture(
                    &context,
                    array
                        .get(2)
                        .cloned()
                        .ok_or_else(|| {
                            JsValue::from_str(&format!(
                                "Couldn't convert value to ImageBitmap: {:?}",
                                path
                            ))
                        })?
                        .dyn_into::<ImageBitmap>()?,
                );
                console_log!("Loaded {}", path);
                ret
            } else {
                Err(JsValue::from_str("Couldn't find texture"))
            }
        };

        Ok(Assets {
            world_transform: Matrix4::from_translation(Vector3::new(-1., 1., 0.))
                * Matrix4::from_nonuniform_scale(2. / FWIDTH, -2. / FHEIGHT, 1.),
            tex_dirt: load_texture_local("dirt")?,
            tex_iron: load_texture_local("iron")?,
            tex_copper: load_texture_local("copper")?,
            tex_coal: load_texture_local("coal")?,
            tex_stone: load_texture_local("stone")?,
            tex_back: load_texture_local("backTiles")?,
            sprite_shader: None,
            textured_shader: None,
            rect_buffer: None,
        })
    }
}
