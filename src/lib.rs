extern crate imgui;

use imgui::{ImGui,Ui};
use std::mem;

mod gl {
  #![cfg_attr(feature = "cargo-clippy", allow(unreadable_literal, too_many_arguments))]

  include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use gl::types::*;

pub struct Renderer {
  gl: gl::Gl,
  program: GLuint,
  locs: Locs,
  vbo: GLuint,
  ebo: GLuint,
  font_texture: GLuint,
}

struct Locs {
  texture: GLint,
  proj_mtx: GLint,
  position: GLuint,
  uv: GLuint,
  color: GLuint,
}

impl Renderer {
  pub fn new<F>(
    imgui: &mut ImGui,
    load_fn: F,
  ) -> Self
    where
    F: FnMut(&'static str) -> *const ::std::os::raw::c_void
  {
    let gl = gl::Gl::load_with(load_fn);

    unsafe {
      let vert_source = b"
        #version 150
        uniform mat4 ProjMtx;
        in vec2 Position;
        in vec2 UV;
        in vec4 Color;
        out vec2 Frag_UV;
        out vec4 Frag_Color;
        void main()
        {
          Frag_UV = UV;
          Frag_Color = Color;
          gl_Position = ProjMtx * vec4(Position.xy,0,1);
        }
      \0";

      let frag_source = b"
        #version 150
        uniform sampler2D Texture;
        in vec2 Frag_UV;
        in vec4 Frag_Color;
        out vec4 Out_Color;
        void main()
        {
          Out_Color = Frag_Color * texture( Texture, Frag_UV.st);
        }
      \0";

      let program = gl.CreateProgram();
      let vert_shader = gl.CreateShader(gl::VERTEX_SHADER);
      let frag_shader = gl.CreateShader(gl::FRAGMENT_SHADER);
      gl.ShaderSource(vert_shader, 1, &(vert_source.as_ptr() as *const GLchar), &(vert_source.len() as GLint));
      gl.ShaderSource(frag_shader, 1, &(frag_source.as_ptr() as *const GLchar), &(frag_source.len() as GLint));
      gl.CompileShader(vert_shader);
      gl.CompileShader(frag_shader);
      gl.AttachShader(program, vert_shader);
      gl.AttachShader(program, frag_shader);
      gl.LinkProgram(program);
      gl.DeleteShader(vert_shader);
      gl.DeleteShader(frag_shader);

      let locs = Locs{
        texture: gl.GetUniformLocation(program, b"Texture\0".as_ptr() as _),
        proj_mtx: gl.GetUniformLocation(program, b"ProjMtx\0".as_ptr() as _),
        position: gl.GetAttribLocation(program, b"Position\0".as_ptr() as _) as _,
        uv: gl.GetAttribLocation(program, b"UV\0".as_ptr() as _) as _,
        color: gl.GetAttribLocation(program, b"Color\0".as_ptr() as _) as _,
      };

      let vbo = return_param(|x| gl.GenBuffers(1, x) );
      let ebo = return_param(|x| gl.GenBuffers(1, x) );

      let mut current_texture = 0;
      gl.GetIntegerv(gl::TEXTURE_BINDING_2D, &mut current_texture);


      let font_texture = return_param(|x| gl.GenTextures(1, x));
      gl.BindTexture(gl::TEXTURE_2D, font_texture);
      gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
      gl.TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
      gl.PixelStorei(gl::UNPACK_ROW_LENGTH, 0);

      imgui.prepare_texture(|handle| {
        gl.TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as _, handle.width as _, handle.height as _, 0, gl::RGBA, gl::UNSIGNED_BYTE, handle.pixels.as_ptr() as _);
      });

      gl.BindTexture(gl::TEXTURE_2D, current_texture as _);

      imgui.set_texture_id(font_texture as usize);

      Self{
        gl,
        program,
        locs,
        vbo,
        ebo,
        font_texture,
      }
    }
  }

  pub fn render<'ui>(
    &self,
    ui: Ui<'ui>,
  ) {
    use imgui::{ImDrawVert,ImDrawIdx};

    let gl = &self.gl;

    unsafe {
      let last_active_texture = return_param(|x| gl.GetIntegerv(gl::ACTIVE_TEXTURE, x));
      gl.ActiveTexture(gl::TEXTURE0);
      let last_program = return_param(|x| gl.GetIntegerv(gl::CURRENT_PROGRAM, x));
      let last_texture = return_param(|x| gl.GetIntegerv(gl::TEXTURE_BINDING_2D, x));
      let last_sampler = if gl.BindSampler.is_loaded() { return_param(|x| gl.GetIntegerv(gl::SAMPLER_BINDING, x)) } else { 0 };
      let last_array_buffer = return_param(|x| gl.GetIntegerv(gl::ARRAY_BUFFER_BINDING, x));
      let last_element_array_buffer = return_param(|x| gl.GetIntegerv(gl::ELEMENT_ARRAY_BUFFER_BINDING, x));
      let last_vertex_array = return_param(|x| gl.GetIntegerv(gl::VERTEX_ARRAY_BINDING, x));
      let last_polygon_mode = return_param(|x: &mut [GLint; 2]| gl.GetIntegerv(gl::POLYGON_MODE, x.as_mut_ptr()));
      let last_viewport = return_param(|x: &mut [GLint; 4]| gl.GetIntegerv(gl::VIEWPORT, x.as_mut_ptr()));
      let last_scissor_box = return_param(|x: &mut [GLint; 4]| gl.GetIntegerv(gl::SCISSOR_BOX, x.as_mut_ptr()));
      let last_blend_src_rgb = return_param(|x| gl.GetIntegerv(gl::BLEND_SRC_RGB, x));
      let last_blend_dst_rgb = return_param(|x| gl.GetIntegerv(gl::BLEND_DST_RGB, x));
      let last_blend_src_alpha = return_param(|x| gl.GetIntegerv(gl::BLEND_SRC_ALPHA, x));
      let last_blend_dst_alpha = return_param(|x| gl.GetIntegerv(gl::BLEND_DST_ALPHA, x));
      let last_blend_equation_rgb = return_param(|x| gl.GetIntegerv(gl::BLEND_EQUATION_RGB, x));
      let last_blend_equation_alpha = return_param(|x| gl.GetIntegerv(gl::BLEND_EQUATION_ALPHA, x));
      let last_enable_blend = gl.IsEnabled(gl::BLEND) == gl::TRUE;
      let last_enable_cull_face = gl.IsEnabled(gl::CULL_FACE) == gl::TRUE;
      let last_enable_depth_test = gl.IsEnabled(gl::DEPTH_TEST) == gl::TRUE;
      let last_enable_scissor_test = gl.IsEnabled(gl::SCISSOR_TEST) == gl::TRUE;


      gl.Enable(gl::BLEND);
      gl.BlendEquation(gl::FUNC_ADD);
      gl.BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
      gl.Disable(gl::CULL_FACE);
      gl.Disable(gl::DEPTH_TEST);
      gl.Enable(gl::SCISSOR_TEST);
      gl.PolygonMode(gl::FRONT_AND_BACK, gl::FILL);



      let (width, height) = ui.imgui().display_size();
      gl.Viewport(0, 0, width as _, height as _);
      let matrix = [
        [ 2.0 / width as f32, 0.0,                     0.0, 0.0],
        [ 0.0,                2.0 / -(height as f32),  0.0, 0.0],
        [ 0.0,                0.0,                    -1.0, 0.0],
        [-1.0,                1.0,                     0.0, 1.0],
      ];
      gl.UseProgram(self.program);
      gl.Uniform1i(self.locs.texture, 0);
      gl.UniformMatrix4fv(self.locs.proj_mtx, 1, gl::FALSE, matrix.as_ptr() as _);
      if gl.BindSampler.is_loaded() { gl.BindSampler(0, 0); }


      let vao = return_param(|x| gl.GenVertexArrays(1, x));
      gl.BindVertexArray(vao);
      gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);
      gl.EnableVertexAttribArray(self.locs.position);
      gl.EnableVertexAttribArray(self.locs.uv);
      gl.EnableVertexAttribArray(self.locs.color);
      gl.VertexAttribPointer(self.locs.position, 2, gl::FLOAT,         gl::FALSE, mem::size_of::<ImDrawVert>() as _, field_offset::<ImDrawVert, _, _>(|v| &v.pos) as _);
      gl.VertexAttribPointer(self.locs.uv,       2, gl::FLOAT,         gl::FALSE, mem::size_of::<ImDrawVert>() as _, field_offset::<ImDrawVert, _, _>(|v| &v.uv) as _);
      gl.VertexAttribPointer(self.locs.color,    4, gl::UNSIGNED_BYTE, gl::TRUE,  mem::size_of::<ImDrawVert>() as _, field_offset::<ImDrawVert, _, _>(|v| &v.col) as _);


      ui.render::<_, ()>(|_ui, draw_data| {
        for draw_list in &draw_data {
          gl.BindBuffer(gl::ARRAY_BUFFER, self.vbo);
          gl.BufferData(gl::ARRAY_BUFFER, (draw_list.vtx_buffer.len() * mem::size_of::<ImDrawVert>()) as _, draw_list.vtx_buffer.as_ptr() as _, gl::STREAM_DRAW);

          gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.ebo);
          gl.BufferData(gl::ELEMENT_ARRAY_BUFFER, (draw_list.idx_buffer.len() * mem::size_of::<ImDrawIdx>()) as _, draw_list.idx_buffer.as_ptr() as _, gl::STREAM_DRAW);

          let mut idx_start = 0;
          for cmd in draw_list.cmd_buffer {
            if let Some(_callback) = cmd.user_callback {
              unimplemented!("Haven't implemented user callbacks yet");
            } else {
              gl.BindTexture(gl::TEXTURE_2D, cmd.texture_id as _);
              gl.Scissor(cmd.clip_rect.x as GLint,
                         (height - cmd.clip_rect.w) as GLint,
                         (cmd.clip_rect.z - cmd.clip_rect.x) as GLint,
                         (cmd.clip_rect.w - cmd.clip_rect.y) as GLint);
              gl.DrawElements(gl::TRIANGLES, cmd.elem_count as _, if mem::size_of::<ImDrawIdx>() == 2 { gl::UNSIGNED_SHORT } else { gl::UNSIGNED_INT }, idx_start as _);
            }
            idx_start += cmd.elem_count * mem::size_of::<ImDrawIdx>() as u32;
          }
        }

        Ok(())
      }).unwrap();


      gl.UseProgram(last_program as _);
      gl.BindTexture(gl::TEXTURE_2D, last_texture as _);
      if gl.BindSampler.is_loaded() { gl.BindSampler(0, last_sampler as _); }
      gl.ActiveTexture(last_active_texture as _);
      gl.BindVertexArray(last_vertex_array as _);
      gl.BindBuffer(gl::ARRAY_BUFFER, last_array_buffer as _);
      gl.BindBuffer(gl::ELEMENT_ARRAY_BUFFER, last_element_array_buffer as _);
      gl.BlendEquationSeparate(last_blend_equation_rgb as _, last_blend_equation_alpha as _);
      gl.BlendFuncSeparate(last_blend_src_rgb as _, last_blend_dst_rgb as _, last_blend_src_alpha as _, last_blend_dst_alpha as _);
      if last_enable_blend { gl.Enable(gl::BLEND) } else { gl.Disable(gl::BLEND) };
      if last_enable_cull_face { gl.Enable(gl::CULL_FACE) } else { gl.Disable(gl::CULL_FACE) };
      if last_enable_depth_test { gl.Enable(gl::DEPTH_TEST) } else { gl.Disable(gl::DEPTH_TEST) };
      if last_enable_scissor_test { gl.Enable(gl::SCISSOR_TEST) } else { gl.Disable(gl::SCISSOR_TEST) };
      gl.PolygonMode(gl::FRONT_AND_BACK, last_polygon_mode[0] as _);
      gl.Viewport(last_viewport[0] as _, last_viewport[1] as _, last_viewport[2] as _, last_viewport[3] as _);
      gl.Scissor(last_scissor_box[0] as _, last_scissor_box[1] as _, last_scissor_box[2] as _,  last_scissor_box[3] as _);

    }
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
    let gl = &self.gl;

    unsafe {
      gl.DeleteBuffers(1, &self.vbo);
      gl.DeleteBuffers(1, &self.ebo);

      gl.DeleteProgram(self.program);

      gl.DeleteTextures(1, &self.font_texture);
    }
  }
}

fn field_offset<T, U, F: for<'a> FnOnce(&'a T) -> &'a U>(f: F) -> usize {
  unsafe {
    let instance = mem::uninitialized::<T>();

    let offset = {
      let field: &U = f(&instance);
      field as *const U as usize - &instance as *const T as usize
    };

    mem::forget(instance);

    offset
  }
}

fn return_param<T, F>(f: F) -> T where F: FnOnce(&mut T) {
  let mut val = unsafe{ mem::uninitialized() };
  f(&mut val);
  val
}
