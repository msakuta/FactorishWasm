use super::{shader_bundle::ShaderBundle, utils::load_texture};
use cgmath::{Matrix4, Vector3};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{ImageBitmap, WebGlBuffer, WebGlRenderingContext as GL, WebGlTexture};

const FWIDTH: f64 = 100.;
const FHEIGHT: f64 = 100.;

pub(crate) struct Assets {
    pub world_transform: Matrix4<f64>,

    pub back_tex: Rc<WebGlTexture>,

    pub sprite_shader: Option<ShaderBundle>,
    pub textured_shader: Option<ShaderBundle>,

    pub rect_buffer: Option<WebGlBuffer>,
}

impl Assets {
    pub fn new(context: &GL, image_assets: js_sys::Array) -> Result<Self, JsValue> {
        let load_texture_local = |path| -> Result<Rc<WebGlTexture>, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                load_texture(
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
                )
            } else {
                Err(JsValue::from_str("Couldn't find texture"))
            }
        };

        Ok(Assets {
            world_transform: Matrix4::from_translation(Vector3::new(-1., 1., 0.))
                * Matrix4::from_nonuniform_scale(2. / FWIDTH, -2. / FHEIGHT, 1.),
            back_tex: load_texture_local("dirt")?,
            sprite_shader: None,
            textured_shader: None,
            rect_buffer: None,
        })
    }
}
