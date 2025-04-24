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
use tobj;

#[derive(Default)]
pub struct Scene {
  pub materials: Vec<Material>,
  pub vertices: Vec<[f32; 3]>,
  pub texture_coords: Vec<[f32; 2]>,
  pub triangles: Vec<[f32; 3]>,
  pub triangles_indexed: Vec<TriIndexed>,
  pub aabb: AABB,
}

#[derive(Default, Clone)]
pub struct Material {
  pub name: String,
  pub texture: Option<String>,
  pub diffuse: [f32; 3],
  pub specular: [f32; 3],
  pub ambient: [f32; 3],
  pub exponent: f32,
}

#[derive(Default, Clone)]
pub struct TriIndexed {
  pub v_idx: [usize; 3],
  pub tc_idx: [usize; 3],
  pub mat_idx: usize,
}

#[derive(Default)]
pub struct AABB {
  pub min: [f32; 3],
  pub max: [f32; 3],
}

pub fn load_obj_scene(filepath: &str) -> Result<Scene, String> {
  let (models, materials) = tobj::load_obj(
    filepath,
    &tobj::LoadOptions {
      triangulate: true,
      ..Default::default()
    },
  )
  .map_err(|e| format!("Failed to load OBJ file: {e}"))?;

  let mut scene = Scene::default();

  scene.aabb.min = [f32::MAX; 3];
  scene.aabb.max = [f32::MIN; 3];

  let materials_map: HashMap<String, Material> = materials
    .unwrap_or_default()
    .iter()
    .map(|m| {
      let mat = Material {
        name: m.name.clone(),
        texture: m.diffuse_texture.clone(),
        diffuse: m.diffuse.unwrap_or([0.0; 3]),
        specular: m.specular.unwrap_or([0.0; 3]),
        ambient: m.ambient.unwrap_or([0.0; 3]),
        exponent: m.shininess.unwrap_or(0.0),
      };
      (mat.name.clone(), mat)
    })
    .collect();

  scene.materials = materials_map.values().cloned().collect();

  for model in models {
    let mesh = &model.mesh;
    let has_texcoords = !mesh.texcoords.is_empty();

    let mut unique_vertex_map: HashMap<(usize, Option<usize>), usize> = HashMap::new();

    for i in (0..mesh.indices.len()).step_by(3) {
      let mut v_idx = [0usize; 3];
      let mut tc_idx = [0usize; 3];

      for j in 0..3 {
        let pos_idx = mesh.indices[i + j] as usize;
        let tex_idx = if has_texcoords {
          Some(mesh.texcoord_indices[i + j] as usize)
        } else {
          None
        };

        let key = (pos_idx, tex_idx);

        let vertex_id = *unique_vertex_map.entry(key).or_insert_with(|| {
          // Add vertex position
          let pos = [
            mesh.positions[3 * pos_idx],
            mesh.positions[3 * pos_idx + 1],
            mesh.positions[3 * pos_idx + 2],
          ];
          scene.vertices.push(pos);

          for k in 0..3 {
            scene.aabb.min[k] = scene.aabb.min[k].min(pos[k]);
            scene.aabb.max[k] = scene.aabb.max[k].max(pos[k]);
          }

          if let Some(ti) = tex_idx {
            let uv = [mesh.texcoords[2 * ti], mesh.texcoords[2 * ti + 1]];
            scene.texture_coords.push(uv);
          } else {
            scene.texture_coords.push([0.0, 0.0]); // placeholder
          }

          scene.vertices.len() - 1
        });

        v_idx[j] = vertex_id;
        tc_idx[j] = vertex_id; // Match by vertex_id, since texcoords are packed the same
      }

      let mat_idx = mesh.material_id.unwrap_or(0) as usize;

      scene.triangles_indexed.push(TriIndexed {
        v_idx,
        tc_idx,
        mat_idx,
      });

      scene.triangles.push(scene.vertices[v_idx[0]]);
      scene.triangles.push(scene.vertices[v_idx[1]]);
      scene.triangles.push(scene.vertices[v_idx[2]]);
    }
  }

  Ok(scene)
}

