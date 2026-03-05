use std::collections::HashMap;

use glow::HasContext;

pub struct Shader {
    program: glow::Program,
    uniforms: HashMap<String, glow::UniformLocation>,
}

impl Shader {
    pub fn new(gl: &glow::Context, vert_source: &str, frag_source: &str) -> Result<Self, String> {
        unsafe {
            let program = gl.create_program().map_err(|e| e.to_string())?;

            let vert = gl.create_shader(glow::VERTEX_SHADER).map_err(|e| e.to_string())?;
            gl.shader_source(vert, vert_source);
            gl.compile_shader(vert);
            if !gl.get_shader_compile_status(vert) {
                let info = gl.get_shader_info_log(vert);
                gl.delete_shader(vert);
                gl.delete_program(program);
                return Err(format!("Vertex shader compile error: {}", info));
            }
            gl.attach_shader(program, vert);

            let frag = gl.create_shader(glow::FRAGMENT_SHADER).map_err(|e| e.to_string())?;
            gl.shader_source(frag, frag_source);
            gl.compile_shader(frag);
            if !gl.get_shader_compile_status(frag) {
                let info = gl.get_shader_info_log(frag);
                gl.delete_shader(vert);
                gl.delete_shader(frag);
                gl.delete_program(program);
                return Err(format!("Fragment shader compile error: {}", info));
            }
            gl.attach_shader(program, frag);

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                let info = gl.get_program_info_log(program);
                gl.delete_program(program);
                return Err(format!("Shader link error: {}", info));
            }

            gl.delete_shader(vert);
            gl.delete_shader(frag);

            Ok(Self {
                program,
                uniforms: HashMap::new(),
            })
        }
    }

    pub fn bind(&self, gl: &glow::Context) {
        unsafe {
            gl.use_program(Some(self.program));
        }
    }

    pub fn unbind(gl: &glow::Context) {
        unsafe {
            gl.use_program(None);
        }
    }

    fn get_uniform(&mut self, gl: &glow::Context, name: &str) -> Option<glow::UniformLocation> {
        if let Some(loc) = self.uniforms.get(name) {
            return Some(*loc);
        }
        unsafe {
            let loc = gl.get_uniform_location(self.program, name);
            if let Some(l) = loc {
                self.uniforms.insert(name.to_string(), l);
            }
            loc
        }
    }

    pub fn set_mat4(&mut self, gl: &glow::Context, name: &str, matrix: &[f32; 16]) {
        if let Some(loc) = self.get_uniform(gl, name) {
            unsafe {
                gl.uniform_matrix_4_f32_slice(Some(&loc), false, matrix);
            }
        }
    }

    pub fn set_1f(&mut self, gl: &glow::Context, name: &str, value: f32) {
        if let Some(loc) = self.get_uniform(gl, name) {
            unsafe {
                gl.uniform_1_f32(Some(&loc), value);
            }
        }
    }

    pub fn set_1iv(&mut self, gl: &glow::Context, name: &str, values: &[i32]) {
        if let Some(loc) = self.get_uniform(gl, name) {
            unsafe {
                gl.uniform_1_i32_slice(Some(&loc), values);
            }
        }
    }

    pub fn set_1fv(&mut self, gl: &glow::Context, name: &str, values: &[f32]) {
        if let Some(loc) = self.get_uniform(gl, name) {
            unsafe {
                gl.uniform_1_f32_slice(Some(&loc), values);
            }
        }
    }

    pub fn bind_texture(
        &mut self,
        gl: &glow::Context,
        texture: glow::Texture,
        unit: u32,
        name: &str,
    ) {
        if let Some(loc) = self.get_uniform(gl, name) {
            unsafe {
                gl.active_texture(glow::TEXTURE0 + unit);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.tex_parameter_i32(
                    glow::TEXTURE_2D,
                    glow::TEXTURE_MIN_FILTER,
                    glow::LINEAR as i32,
                );
                gl.uniform_1_i32(Some(&loc), unit as i32);
            }
        }
    }

}
