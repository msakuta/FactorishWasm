use web_sys::{WebGlProgram, WebGlRenderingContext as GL, WebGlUniformLocation};

pub(crate) struct ShaderBundle {
    pub program: WebGlProgram,
    pub vertex_position: u32,
    #[allow(dead_code)] // May use later
    pub tex_coord_position: u32,
    pub texture_loc: Option<WebGlUniformLocation>,
    pub transform_loc: Option<WebGlUniformLocation>,
    pub tex_transform_loc: Option<WebGlUniformLocation>,
    pub attrib_position_loc: i32,
    #[allow(dead_code)] // May use later
    pub alpha_loc: Option<WebGlUniformLocation>,
    pub color_loc: Option<WebGlUniformLocation>,
}

impl ShaderBundle {
    pub fn new(gl: &GL, program: WebGlProgram) -> Self {
        let get_uniform = |location: &str| {
            let op: Option<WebGlUniformLocation> = gl.get_uniform_location(&program, location);
            if op.is_none() {
                console_log!("Warning: location {} undefined", location);
            } else {
                console_log!("location {} defined", location);
            }
            op
        };
        let vertex_position = gl.get_attrib_location(&program, "vertexData") as u32;
        let tex_coord_position = gl.get_attrib_location(&program, "texCoord") as u32;
        console_log!("vertex_position: {}", vertex_position);
        console_log!("tex_coord_position: {}", tex_coord_position);
        let attrib_position_loc = gl.get_attrib_location(&program, "position");
        Self {
            vertex_position,
            tex_coord_position,
            texture_loc: get_uniform("texture"),
            transform_loc: get_uniform("transform"),
            tex_transform_loc: get_uniform("texTransform"),
            alpha_loc: get_uniform("alpha"),
            color_loc: get_uniform("color"),
            attrib_position_loc,
            // Program has to be later than others
            program,
        }
    }
}
