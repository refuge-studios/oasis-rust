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

use std::collections::HashMap;
use std::env;
use std::ffi::CString;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::slice;

use image::DynamicImage;
use image::GenericImageView;

mod scene_loader;
use scene_loader::load_obj_scene;
use scene_loader::Scene;

use oasis_bindings::*;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Node {
  pub children: [i32; 8],
  pub yuv: [f32; 4],
}

#[repr(C)]
#[derive(Debug)]
pub struct NodePool {
  pub nodes: *const Node,
  pub count: usize,
}

pub fn serialize_node_pool<P: AsRef<Path>>(node_pool: &NodePool, path: P) -> io::Result<()> {
  if node_pool.nodes.is_null() || node_pool.count == 0 {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, "Node pool is empty or null"));
  }

  let node_slice = unsafe { slice::from_raw_parts(node_pool.nodes, node_pool.count) };

  let mut file = File::create(path)?;

  // Write node count
  file.write_all(&(node_pool.count as u64).to_le_bytes())?;

  // Write node data
  let byte_slice = unsafe {
    slice::from_raw_parts(
      node_slice.as_ptr() as *const u8,
      node_slice.len() * std::mem::size_of::<Node>(),
    )
  };

  file.write_all(byte_slice)?;
  Ok(())
}


pub fn load_textures(scene: &Scene, obj_file_path: &Path, c_scene: oasis_scene_t) -> Result<(), Box<dyn std::error::Error>> {
  let obj_dir = obj_file_path.parent().expect("OBJ file must be in a directory");

  let mut loaded_textures: HashMap<String, Vec<u8>> = HashMap::new();

  for material in &scene.materials {
    if let Some(ref texture_name) = material.texture {
      if loaded_textures.contains_key(texture_name) {
        continue;
      }

      let texture_path = obj_dir.join(texture_name);
      println!("Loading and flipping texture '{}' for material '{}'...", texture_name, material.name);

      // Load and flip image vertically
      let img: DynamicImage = image::open(&texture_path)?.flipv().to_rgb8().into();
      let (width, height) = img.dimensions();
      let data = img.into_rgb8().into_raw();

      loaded_textures.insert(texture_name.clone(), data.clone());

      let c_name = CString::new(texture_name.as_str())?;
      unsafe {
        oasis_scene_add_texture(
          c_scene,
          c_name.as_ptr(),
          data.as_ptr(),
          width as i32,
          height as i32,
          3,
        );
      }
    }
  }

  Ok(())
}

fn main() {
  // Parse the command-line arguments
  let args: Vec<String> = env::args().collect();

  // Ensure at least 3 arguments (the program name, obj_file, depth, and step level)
  if args.len() < 4 {
    eprintln!("Usage: ./builder <model.obj> <depth> <step_level> [output_name]");
    std::process::exit(1);
  }

  // The .obj file to load
  let obj_file = &args[1];

  // Parse the depth and step level
  let depth: u8 = args[2].parse().expect("Invalid depth argument");
  let step_level: u8 = args[3].parse().expect("Invalid step level argument");

  // Handle the optional output file name argument
  let output_name = if args.len() > 4 {
    &args[4]
  } else {
    "out"  // Provide a default output name if not given
  };

  let scene = match load_obj_scene(&obj_file) {
    Ok(scene) => {
      println!("OBJ file loaded successfully!");
      scene
    }
    Err(e) => {
      eprintln!("Error loading OBJ file: {}", e);
      return;
    }
  };

  unsafe {
    let c_scene = oasis_scene_create();
    
    oasis_scene_set_vertices(
      c_scene,
      scene.vertices.as_ptr() as *const vec3f_t,
      scene.vertices.len(),
    );
    
    oasis_scene_set_tex_coords(
      c_scene,
      scene.texture_coords.as_ptr() as *const vec2f_t,
      scene.texture_coords.len(),
    );

    oasis_scene_set_raw_triangles(
      c_scene,
      scene.triangles.as_ptr() as *const vec3f_t,
      scene.triangles.len(),
    );

    oasis_scene_set_indexed_triangles(
      c_scene,
      scene.triangles_indexed.as_ptr() as *const tri_indexed_c_t,
      scene.triangles_indexed.len(),
    );

    let bbox = bbox_c_t {
      min: scene.aabb.min,
      max: scene.aabb.max,
    };
    oasis_scene_set_aabb(c_scene, &bbox);

    let obj_path = Path::new(&obj_file);

    for mat in &scene.materials {
      let name_cstr = CString::new(mat.name.clone()).expect("Invalid material name");
      let mat_c = material_c_t {
        name: name_cstr.as_ptr(),
        texture: mat.texture.as_ref().map_or(std::ptr::null(), |s| s.as_ptr()) as *const i8,
        diffuse: mat.diffuse,
        specular: mat.specular,
        ambient: mat.ambient,
        exponent: mat.exponent,
        transparancy: 1.0,
      };
      oasis_scene_add_material(c_scene, &mat_c);
    }

    if let Err(e) = load_textures(&scene, obj_path, c_scene) {
      eprintln!("Error loading textures: {}", e);
    }

    let builder = oasis_node_pool_builder_create();
    assert!(!builder.is_null(), "Failed to create builder");

    oasis_node_pool_builder_build(builder, c_scene, depth, step_level);

    let pool_handle = oasis_node_pool_builder_get_pool(builder);
    assert!(!pool_handle.is_null(), "Failed to get pool handle");

    let pool_ptr = oasis_node_pool_get(pool_handle);
    assert!(!pool_ptr.is_null(), "Failed to get pool pointer");

    let node_pool: &NodePool = &*(pool_ptr as *const NodePool);
    println!("Serializing pool: count = {},", node_pool.count);

    serialize_node_pool(node_pool, output_name.to_string() + ".svdag").expect("Failed to serialize node pool");
        
    // Destroy builder first
    oasis_node_pool_builder_destroy(builder);
    // Destroy and Free
    oasis_node_pool_free(pool_ptr); 
    // Destroy Scene
    oasis_scene_destroy(c_scene);
  }
}
