/*
 * Example Code for the Oasis Graphics Framework
 * Copyright (c) 2025 REFUGE STUDIOS PTY LTD.
 * Created by Aidan Sanders <aidan.sanders@refugestudios.com.au>
 *
 * This example code is licensed under the MIT License.
 * You are free to use, modify, and distribute this code for any purpose,
 * including commercial applications, as long as this notice is retained.
 *
 * THE OASIS API ITSELF IS PROPRIETARY AND NOT COVERED UNDER THIS LICENSE.
 * These examples are intended to demonstrate usage of the Oasis API,
 * and require a licensed copy of Oasis to function.
 *
 * For licensing Oasis itself, please contact: aidan.sanders@refugestudios.com.au
 */

// OpenGL and Window
extern crate glfw;
use self::glfw::{Context, Key, Action};

extern crate gl;
use self::gl::types::*;

use std::sync::mpsc::Receiver;
use std::ffi::CString;
use std::ptr;
use std::str;
use std::mem;
use std::os::raw::c_void;
use std::slice;
use std::env;

use oasis_bindings::*;

// Camera
use nalgebra_glm as glm;

// settings
const SCR_WIDTH: u32 = 800;
const SCR_HEIGHT: u32 = 600;


pub struct Camera {
  pub position: glm::Vec3,
  pub front: glm::Vec3,
  pub up: glm::Vec3,
  pub right: glm::Vec3,
  pub world_up: glm::Vec3,
  pub yaw: f32,
  pub pitch: f32,
  pub fov: f32,
  pub aspect_ratio: f32,
  pub near: f32,
  pub far: f32,
}

impl Camera {
  pub fn new(position: glm::Vec3, aspect_ratio: f32) -> Self {
    let mut camera = Self {
      position,
      front: glm::vec3(0.0, 0.0, -1.0),
      up: glm::vec3(0.0, 1.0, 0.0),
      right: glm::vec3(1.0, 0.0, 0.0),
      world_up: glm::vec3(0.0, 1.0, 0.0),
      yaw: -90.0,
      pitch: 0.0,
      fov: 45.0,
      aspect_ratio,
      near: 0.1,
      far: 100.0,
    };
    camera.update_vectors();
    camera
  }

  pub fn get_view_matrix(&self) -> glm::Mat4 {
    glm::look_at(&self.position, &(self.position + self.front), &self.up)
  }

  pub fn get_proj_matrix(&self) -> glm::Mat4 {
    glm::perspective(self.aspect_ratio, self.fov.to_radians(), self.near, self.far)
  }

  pub fn get_view_proj_matrix(&self) -> glm::Mat4 {
    self.get_proj_matrix() * self.get_view_matrix()
  }

  pub fn update_vectors(&mut self) {
    let yaw_radians = self.yaw.to_radians();
    let pitch_radians = self.pitch.to_radians();

    let front = glm::vec3(
      yaw_radians.cos() * pitch_radians.cos(),
      pitch_radians.sin(),
      yaw_radians.sin() * pitch_radians.cos(),
    );
    self.front = glm::normalize(&front);
    self.right = glm::normalize(&glm::cross(&self.front, &self.world_up));
    self.up = glm::normalize(&glm::cross(&self.right, &self.front));
  }

  pub fn process_mouse_movement(&mut self, x_offset: f32, y_offset: f32, constrain_pitch: bool) {
    let sensitivity = 0.1;
    self.yaw += x_offset * sensitivity;
    self.pitch += y_offset * sensitivity;

    if constrain_pitch {
      if self.pitch > 89.0 {
        self.pitch = 89.0;
      }
      if self.pitch < -89.0 {
        self.pitch = -89.0;
      }
    }

    self.update_vectors();
  }

  pub fn process_keyboard(&mut self, direction: CameraMovement, delta_time: f32) {
    let velocity = 2.5 * delta_time;
    match direction {
      CameraMovement::Forward => self.position += self.front * velocity,
      CameraMovement::Backward => self.position -= self.front * velocity,
      CameraMovement::Left => self.position -= self.right * velocity,
      CameraMovement::Right => self.position += self.right * velocity,
    }
  }
}

pub enum CameraMovement {
  Forward,
  Backward,
  Left,
  Right,
}

fn compile_shader(src: &str, shader_type: GLenum) -> GLuint {
  let shader = unsafe { gl::CreateShader(shader_type) };
  let c_str = CString::new(src).unwrap();
  unsafe {
    gl::ShaderSource(shader, 1, &c_str.as_ptr(), ptr::null());
    gl::CompileShader(shader);

    let mut success = gl::FALSE as GLint;
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
    if success != gl::TRUE as GLint {
      let mut info_log = vec![0; 512];
      gl::GetShaderInfoLog(shader, 512, ptr::null_mut(), info_log.as_mut_ptr() as *mut GLchar);
      panic!(
        "ERROR::SHADER::{:?}::COMPILATION_FAILED\n{}",
        shader_type,
        str::from_utf8(&info_log).unwrap()
      );
    }
  }
  shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
  let program = unsafe { gl::CreateProgram() };
  unsafe {
    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);

    let mut success = gl::FALSE as GLint;
    gl::GetProgramiv(program, gl::LINK_STATUS, &mut success);
    if success != gl::TRUE as GLint {
      let mut info_log = vec![0; 512];
      gl::GetProgramInfoLog(program, 512, ptr::null_mut(), info_log.as_mut_ptr() as *mut GLchar);
      panic!("ERROR::PROGRAM::LINKING_FAILED\n{}", str::from_utf8(&info_log).unwrap());
    }

    gl::DeleteShader(vs);
    gl::DeleteShader(fs);
  }
  program
}

fn create_fullscreen_quad_vao() -> GLuint {
  let vertices: [f32; 12] = [
    -1.0, -1.0, 0.0,
     1.0, -1.0, 0.0,
    -1.0,  1.0, 0.0,
     1.0,  1.0, 0.0,
  ];
  let (mut vbo, mut vao) = (0, 0);
  unsafe {
    gl::GenVertexArrays(1, &mut vao);
    gl::GenBuffers(1, &mut vbo);
    gl::BindVertexArray(vao);

    gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
    gl::BufferData(gl::ARRAY_BUFFER,
      (vertices.len() * mem::size_of::<f32>()) as isize,
      vertices.as_ptr() as *const _,
      gl::STATIC_DRAW
    );

    gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, 0, ptr::null());
    gl::EnableVertexAttribArray(0);

    gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    gl::BindVertexArray(0);
  }
  vao
}

pub fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    eprintln!("Usage: ./viewer <model.obj>");
    std::process::exit(1);
  }

  let filename = &args[1];

  // Convert the obj_file path to a CString
  let c_filename = CString::new(filename.as_str()).unwrap_or_else(|_| {
    eprintln!("Invalid filename: contains a null byte.");
    std::process::exit(1);
  });

  // initialize and configure GLFW
  let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
  glfw.window_hint(glfw::WindowHint::ContextVersion(4, 5));
  glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
  #[cfg(target_os = "macos")]
  glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
 
  let width: u32 = SCR_WIDTH;
  let height: u32 = SCR_HEIGHT;
  
  // GLFW window creation
  let (mut window, events) = glfw.create_window(width, height, "Oasis Viewer (Rust)", glfw::WindowMode::Windowed)
    .expect("Failed to create GLFW window");

  window.make_current();
  window.set_key_polling(true);
  window.set_framebuffer_size_polling(true);
  window.set_cursor_pos_polling(true);
  glfw.set_swap_interval(glfw::SwapInterval::Sync(1)); // Enable V-Sync

  // Load all OpenGL function pointers
  gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);
  



  let handle = unsafe { oasis_node_pool_deserialize(c_filename.as_ptr()) };
  if handle.is_null() {
    panic!("Failed to deserialize node pool.");
  }
  
  let pool_ptr = unsafe { oasis_node_pool_get(handle) };
  if pool_ptr.is_null() {
    panic!("Failed to get node pool.");
  }
  
  let pool = unsafe { &*pool_ptr };
  println!("Node count: {}", pool.count);
  
  let nodes = unsafe {
    slice::from_raw_parts(pool.nodes, pool.count as usize)
  };
  println!("Loaded {} nodes from C.", nodes.len());

  const VERTEX_SHADER_SOURCE: &str = include_str!("vert.glsl");
  const FRAGMENT_SHADER_SOURCE: &str = include_str!("frag.glsl");

  let vs = compile_shader(&VERTEX_SHADER_SOURCE, gl::VERTEX_SHADER);
  let fs = compile_shader(&FRAGMENT_SHADER_SOURCE, gl::FRAGMENT_SHADER);
  let shader_program = link_program(vs, fs);
  
  let vao = create_fullscreen_quad_vao();

  let mut node_ssbo: GLuint = 0;
  unsafe {
    gl::GenBuffers(1, &mut node_ssbo);
    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, node_ssbo);
    gl::BufferData(
      gl::SHADER_STORAGE_BUFFER,
      (nodes.len() * std::mem::size_of::<node_t>()) as GLsizeiptr,
      nodes.as_ptr() as *const c_void,
      gl::STATIC_DRAW,
    );
    gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, node_ssbo);
    gl::BindBuffer(gl::SHADER_STORAGE_BUFFER, 0);
  }

  let mut camera = Camera::new(glm::vec3(0.0, 0.0, 3.0), width as f32 / height as f32);
  
  let mut last_x = SCR_WIDTH as f32 / 2.0;
  let mut last_y = SCR_HEIGHT as f32 / 2.0;
  let mut first_mouse = true;
  
  let mut last_frame: f32 = 0.0;

  let loc_u_pos = CString::new("uPos").unwrap();
  let loc_u_viewproj = CString::new("uViewProj").unwrap();
  let loc_u_width = CString::new("uWidth").unwrap();
  let loc_u_height = CString::new("uHeight").unwrap();
  
  let u_pos_loc = unsafe { gl::GetUniformLocation(shader_program, loc_u_pos.as_ptr()) };
  let u_viewproj_loc = unsafe { gl::GetUniformLocation(shader_program, loc_u_viewproj.as_ptr()) };
  let u_width_loc = unsafe { gl::GetUniformLocation(shader_program, loc_u_width.as_ptr()) };
  let u_height_loc = unsafe { gl::GetUniformLocation(shader_program, loc_u_height.as_ptr()) };
  
  let mut tab_pressed_last_frame = false; 
  let mut cursor_disabled = true;
  
  // Render loop
  while !window.should_close() {
    let current_frame = glfw.get_time() as f32;
    let delta_time = current_frame - last_frame;
    last_frame = current_frame;

    // Events
    process_events(&mut window, &events);

    // Toggle cursor mode with Tab key
    if window.get_key(Key::Tab) == Action::Press && !tab_pressed_last_frame {
      cursor_disabled = !cursor_disabled;
      window.set_cursor_mode(if cursor_disabled {
        glfw::CursorMode::Disabled
      } else {
        glfw::CursorMode::Normal
      });
      first_mouse = true; // reset on mode change
    }
    tab_pressed_last_frame = window.get_key(Key::Tab) == Action::Press;

    // Camera Movement
    if window.get_key(Key::W) == Action::Press {
      camera.process_keyboard(CameraMovement::Forward, delta_time);
    }
    if window.get_key(Key::S) == Action::Press {
      camera.process_keyboard(CameraMovement::Backward, delta_time);
    }
    if window.get_key(Key::A) == Action::Press {
      camera.process_keyboard(CameraMovement::Left, delta_time);
    }
    if window.get_key(Key::D) == Action::Press {
      camera.process_keyboard(CameraMovement::Right, delta_time);
    }

    // Camera Cursor
    let (xpos, ypos) = window.get_cursor_pos();
    let xpos = xpos as f32;
    let ypos = ypos as f32;

    let (xoffset, yoffset) = if first_mouse {
      first_mouse = false;
      (0.0, 0.0)
    } else {
      (xpos - last_x, last_y - ypos) // y is reversed
    };

    last_x = xpos;
    last_y = ypos;

    camera.process_mouse_movement(xoffset, yoffset, true);

    // Render
    unsafe {
      gl::ClearColor(0.2, 0.3, 0.3, 1.0);
      gl::Clear(gl::COLOR_BUFFER_BIT);
  
      gl::Uniform3f(u_pos_loc, camera.position.x, camera.position.y, camera.position.z);
      let inv_view_proj = glm::inverse(&camera.get_view_proj_matrix());
      gl::UniformMatrix4fv(u_viewproj_loc, 1, gl::FALSE, inv_view_proj.as_ptr());
      gl::Uniform1ui(u_width_loc, width);
      gl::Uniform1ui(u_height_loc, height);
      gl::BindBufferBase(gl::SHADER_STORAGE_BUFFER, 3, node_ssbo);

      // Draw the fullscreen quad
      gl::UseProgram(shader_program);
      gl::BindVertexArray(vao);
      gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
    }

    window.swap_buffers();
    glfw.poll_events();
  }
  
  // Cleanup
  unsafe {
    oasis_node_pool_destroy(handle);
    gl::DeleteBuffers(1, &node_ssbo);
  }
}

fn process_events(window: &mut glfw::Window, events: &Receiver<(f64, glfw::WindowEvent)>) {
  for (_, event) in glfw::flush_messages(events) {
    match event {
      glfw::WindowEvent::FramebufferSize(width, height) => {
        unsafe { gl::Viewport(0, 0, width, height) }
      }
      glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
      _ => {}
    }
  }
}