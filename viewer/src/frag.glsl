
#version 460 core

// Define the maximum depth of the DAG (Directed Acyclic Graph) 
// representation of the voxel octree
#ifndef MAX_DAG_DEPTH
  #define MAX_DAG_DEPTH 23 
#endif

// Level of Detail (LOD) cutoff constant used for ray termination
#ifndef LOD_CUTOFF_CONSTANT
  #define LOD_CUTOFF_CONSTANT 0.02
#endif

// Bit masks for spatial axis encoding
#define AXIS_X_MASK 0x1
#define AXIS_Y_MASK 0x2
#define AXIS_Z_MASK 0x4

const uvec3 AXIS_MASK_VEC = uvec3(AXIS_X_MASK, AXIS_Y_MASK, AXIS_Z_MASK);

// Enumeration for ray-plane intersection incidence
#define INCIDENCE_X 0x0
#define INCIDENCE_Y 0x1
#define INCIDENCE_Z 0x2

// Node structure defining the hierarchical voxel DAG
// - If sub-DAG: Positive 1-indexed pointers to child nodes
// - If leaf: Negative Value
// - If empty: 0 (no voxel data)

struct DAGNode { int children[8]; vec3 yuv; };
layout(std430,binding = 3) buffer uuDAG { DAGNode uDAG[]; };

layout(binding = 1) uniform sampler2D uBeam;

uniform vec3 uPos;    // Camera world position
uniform mat4 uViewProj; // Inversed

uniform uint uWidth;  // Viewport width in pixels
uniform uint uHeight; // Viewport height in pixels

// Output fragment color
out vec4 oColor;

uint idot(uvec3 a, uvec3 b) {
  return uint(dot(a,b));
}

vec2 project_cube(vec3 id, vec3 od, vec3 mn, vec3 mx, out uint incidence_min, out uint incidence_max) {
  vec3 tmn = fma(id, mn, od);
  vec3 tmx = fma(id, mx, od);

  float ts = max(tmn.x, max(tmn.y, tmn.z));
  float te = min(tmx.x, min(tmx.y, tmx.z));

  if (te == tmx.x) {incidence_max = INCIDENCE_X;}
  if (te == tmx.y) {incidence_max = INCIDENCE_Y;}
  if (te == tmx.z) {incidence_max = INCIDENCE_Z;}

  return vec2(ts, te);
}

bool voxel_valid_bit(uint parent, uint idx) {
  return uDAG[parent].children[idx] != 0;
}

#define SUBVOXEL_VALID(sv) (sv != 0)

bool voxel_leaf_bit(uint parent, uint idx) {
  return uDAG[parent].children[idx] < 0;
}

#define SUBVOXEL_LEAF(sv) (sv < 0)

bool voxel_empty(uint parent, uint idx) {
  return uDAG[parent].children[idx] == 0;
}

#define SUBVOXEL_EMPTY(sv) (sv != 0)

uint voxel_get_child(uint parent, uint idx) {
  return uDAG[parent].children[idx] - 1;
}

#define SUBVOXEL_CHILD(sv) (sv - 1)

int voxel_get_subvoxel(uint parent, uint idx) {
  return uDAG[parent].children[idx];
}

uint voxel_get_material(uint parent, uint idx) {
  return -uDAG[parent].children[idx];
}

#define SUBVOXEL_MATERIAL(sv) (-sv)

bool interval_nonempty(vec2 t) {
  return t.x < t.y;
}

vec2 interval_intersect(vec2 a, vec2 b) {
  return vec2(max(a.x,b.x), min(a.y, b.y));
}

uint select_child(vec3 pos, float scale, vec3 o, vec3 d, float t) {
  vec3 p = fma(d, vec3(t), o) - pos - scale;
  uvec3 less = uvec3(lessThan(p, vec3(0)));
  uint idx = 0;
  idx = idot(less, AXIS_MASK_VEC);
  return idx;
}

uint select_child_bit(vec3 pos, float scale, vec3 o, vec3 d, float t) {
  vec3 p = fma(d, vec3(t), o) - pos - scale;
  uvec3 s = uvec3(greaterThan(p, vec3(0)));
  return idot(s, AXIS_MASK_VEC);
}

uvec3 child_cube(uvec3 pos, uint scale, uint idx) {
  uvec3 offset = uvec3(
    bitfieldExtract(idx, 0, 1),
    bitfieldExtract(idx, 1, 1),
    bitfieldExtract(idx, 2, 1)
  );
  return pos + (scale * offset);
}

uint extract_child_slot(uvec3 pos, uint scale) {
  uvec3 d = uvec3(equal(pos & scale, uvec3(0)));
  uint idx = idot(d, AXIS_MASK_VEC);
  return idx;
}

uint extract_child_slot_bfe(uvec3 pos, uint depth) {
  uvec3 d = bitfieldExtract(pos, int(depth), 1);
  return idot(d, AXIS_MASK_VEC);
}

#define VOXEL_MARCH_MISS 0
#define VOXEL_MARCH_HIT 1
#define VOXEL_MARCH_MAX_DEPTH 2
#define VOXEL_MARCH_LOD 3
#define VOXEL_MARCH_MAX_DIST 4
#define VOXEL_MARCH_ERROR 5
#define VOXEL_MARCH_LOOP_END 6

/*
 *  Copyright (c) 2009-2011, NVIDIA Corporation
 *  All rights reserved.
 *
 *  Redistribution and use in source and binary forms, with or without
 *  modification, are permitted provided that the following conditions are met:
 *      * Redistributions of source code must retain the above copyright
 *        notice, this list of conditions and the following disclaimer.
 *      * Redistributions in binary form must reproduce the above copyright
 *        notice, this list of conditions and the following disclaimer in the
 *        documentation and/or other materials provided with the distribution.
 *      * Neither the name of NVIDIA Corporation nor the
 *        names of its contributors may be used to endorse or promote products
 *        derived from this software without specific prior written permission.
 *
 *  THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
 *  ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 *  WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 *  DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
 *  DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
 *  (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
 *  LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
 *  ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
 *  (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
 *  SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */

struct StackItem { uint node; float t_max; } stack[MAX_DAG_DEPTH];
struct ColorItem { vec3 node; } color[MAX_DAG_DEPTH];

bool DAG_RayMarch(vec3 o, vec3 d, 
                  in uint max_depth, 
                  in float max_dist, 
                  out float dist, 
                  out uint incidence, 
                  out uint vid, 
                  out uint material,
                  out vec3 attr,
                  out uint return_state, 
                  out uint iterations)
{
  iterations = 0; 
  const uint MAX_SCALE = (1<<MAX_DAG_DEPTH);

  uint dmask = 0;

  d.x = d.x == 0.0 ? 1e-6 : d.x;
  d.y = d.y == 0.0 ? 1e-6 : d.y;
  d.z = d.z == 0.0 ? 1e-6 : d.z;

  vec3 ds = sign(d);
  d *= ds;
  o = fma(o, ds, (1 - ds) * 0.5);
  o *= MAX_SCALE;
  d *= MAX_SCALE;

  dmask |= ds.x < 0 ? AXIS_X_MASK : 0;
  dmask |= ds.y < 0 ? AXIS_Y_MASK : 0;
  dmask |= ds.z < 0 ? AXIS_Z_MASK : 0;

  vec3 id = 1.0 / d;
  vec3 od = -o * id;

  vec2 t = vec2(0, max_dist);
  float h = t.y;

  uvec3 pos   = ivec3(0);
  uint parent = 0u;
  uint idx    = 0u;
  uint scale  = 1u << MAX_DAG_DEPTH;
  uint depth  = 1u;

  uint incidence_min;
  vec2 tp = project_cube(id, od, pos, pos + scale, incidence_min, incidence);

  t = interval_intersect(t, tp);
  if (!interval_nonempty(t)) {
    // we didn't hit the bounding cube
    return_state = VOXEL_MARCH_MISS;
    dist = tp.x;
    return false;
  }

  scale = scale >> 1;
  idx = select_child_bit(pos, scale, o, d, t.x);
  pos = child_cube(pos, scale, idx);

  return_state = VOXEL_MARCH_MISS;
  
  stack[0].node = parent;
  stack[0].t_max = t.y;

  vec2 tc, tv;

 	// TODO: Each traversal iteration, re-use the attributes of the parent
	// sum up the attributes, divide by the nr of iterations at the end?
	// (later, the luma can have higher influence than chroma)
	for (int i = 0; i < MAX_DAG_DEPTH; i++)
    color[i].node = vec3(0.0);
	

  vec3 attr_sum = vec3(0.0);
  uint attr_count = 0;

  // very hot loop
  while (iterations < 2048) {
    iterations += 1;

    uint new_incidence;
    tc = project_cube(id, od, pos, pos + scale, incidence_min, new_incidence);

    int subvoxel = voxel_get_subvoxel(parent, dmask ^ idx);
    if (SUBVOXEL_VALID(subvoxel) && interval_nonempty(t)) {
      // Subtract the color we added to the attribute sum earlier
      attr_sum -= color[ depth ].node;
		  color[ depth ].node = vec3(0);

      if (scale <= tc.x * LOD_CUTOFF_CONSTANT 
        || depth >= max_depth) {
        // voxel is too small
        dist = t.x;
        return_state = depth >= max_depth ? VOXEL_MARCH_MAX_DEPTH : VOXEL_MARCH_LOD;
        return true;
      }

      if (tc.x > max_dist) {
        // voxel is beyond the render distance
        return_state = VOXEL_MARCH_MAX_DIST;
        return false;
      }

      tv = interval_intersect(tc, t);

      if (interval_nonempty(tv)) {
        if (SUBVOXEL_LEAF(subvoxel)) {
          dist = tv.x;
          vid = (parent << 3) | (dmask ^ idx);
          return_state = VOXEL_MARCH_HIT;
          material = 0;//SUBVOXEL_MATERIAL(subvoxel);
          //vec3 yuv = uDAG[parent].yuv / vec3(255);
          //attr_sum += yuv;
          attr = attr_sum;
          return true;
        }
        // descend:
        if (tc.y < h) {
          stack[ depth ].node = parent;
          stack[ depth ].t_max = t.y;
        }
            
        vec3 yuv = uDAG[parent].yuv / vec3(255.0);
        color[depth].node += yuv;
        attr_sum += yuv;
        
        depth += 1;

        h = tc.y;
        scale = scale >> 1;
        parent = SUBVOXEL_CHILD(subvoxel);
        idx = select_child_bit(pos, scale, o, d, tv.x);
        t = tv;
        pos = child_cube(pos, scale, idx);

        continue;
      }
    }

    incidence = new_incidence;

    // advance
    t.x = tc.y;

    uint mask = 0;
    uint bit_diff = 0;
    uvec3 incidence_mask = uvec3(incidence == INCIDENCE_X, incidence == INCIDENCE_Y, incidence == INCIDENCE_Z);

    bit_diff = idot((pos + scale) ^ pos, incidence_mask);
    pos += scale * incidence_mask;

    mask = (1 << incidence);
    idx ^= mask;
    
    if ((idx & mask) == 0) {
      uint idepth = findMSB(bit_diff);

      // check if we exited voxel tree
      if (idepth >= MAX_DAG_DEPTH) {
        return_state = VOXEL_MARCH_MISS;
        return false;
      }

      depth = MAX_DAG_DEPTH - idepth;

      scale = MAX_SCALE >> depth;

      parent = stack[ depth ].node;
      t.y = stack[ depth ].t_max;

      // round position to correct voxel (mask out low bits)
      // pos &= 0xFFFFFFFF ^ (scale - 1);
      pos = bitfieldInsert(pos, uvec3(0), 0, int(idepth));
      idx = extract_child_slot_bfe(pos, idepth);

      h = 0;
    }
  }
  dist = t.x;
  return_state = VOXEL_MARCH_LOOP_END;
  return false;
}

vec3 GenRay(vec2 uv) {
  vec2 ndc = uv * 2.0 - 1.0;
  vec4 clipSpace = vec4(ndc, 1.0, 1.0);
  vec4 worldSpace = uViewProj * clipSpace;
  worldSpace /= worldSpace.w;
  return normalize(worldSpace.xyz - uPos);
}

#ifdef DEBUG
vec3 Heat(in float x) { return sin(clamp(x, 0.0, 1.0) * 3.0 - vec3(1, 2, 3)) * 0.5 + 0.5; }
#endif 

void main() {
  vec2 coord = gl_FragCoord.xy / vec2(uWidth, uHeight);
	vec3 o = vec3(uPos.x, uPos.y, uPos.z), d = GenRay(coord);

  float max_dist = 100.0;
  uint max_depth = 13;

  float oDist;
  uint oVid;
  uint oIncidence;
  uint oCode;
  uint oIter;
  uint oMaterial;
  vec3 oAttr;
  bool hit = DAG_RayMarch(o, d,
                          max_depth, 
                          max_dist, 
                          oDist, 
                          oIncidence,
                          oVid,
                          oMaterial,
                          oAttr,
                          oCode, 
                          oIter);


  if (hit) {
    ivec3 yuv = ivec3(oAttr * 255.0); 
    
    float Y = float(yuv.x);
    float U = float(yuv.y) - 128.0; 
    float V = float(yuv.z) - 128.0; 

    vec3 color;
    color.r = clamp((Y + 1.13983 * V) / 255.0, 0.0, 1.0);
    color.g = clamp((Y - 0.39465 * U - 0.58060 * V) / 255.0, 0.0, 1.0);
    color.b = clamp((Y + 2.03211 * U) / 255.0, 0.0, 1.0);

    oColor = vec4(color, 1.0);
  }
  else {
    oColor = vec4(0,0,0,1);
  }
#ifdef DEBUG
  oColor = vec4(Heat(oIter / 128.0), 1.0);
#endif
}


