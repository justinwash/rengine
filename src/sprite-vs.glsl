uniform vec2 bl;
uniform vec2 br;
uniform vec2 tr;
uniform vec2 tl;

out vec2 v_uv;
out vec2[4] quad_pos;
out vec2 tex_uv;

void main() {
  vec2[4] QUAD_POS = vec2[](
  bl,
  br,
  tr,
  tl
);

  vec2[4] TEX_QUAD = vec2[](
  vec2(-1., -1.),
  vec2(1., -1.),
  vec2(1., 1.),
  vec2(-1., 1.)
);

  vec2 p = QUAD_POS[gl_VertexID];
  vec2 tex_p = TEX_QUAD[gl_VertexID];

  gl_Position = vec4(p, 0., 1.);
  v_uv = p * .5 + .5; 
  tex_uv = tex_p * .5 + .5;
  quad_pos = QUAD_POS;
}