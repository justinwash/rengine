uniform vec2 bl;
uniform vec2 br;
uniform vec2 tr;
uniform vec2 tl;

out vec2 v_uv;
out vec2[4] quad_pos;

varying vec2 v_texcoord;

void main() {
  vec2[4] QUAD_POS = vec2[](
  bl,
  br,
  tr,
  tl
);

  vec2 p = QUAD_POS[gl_VertexID];

  gl_Position = vec4(p, 0., 1.);
  v_uv = p * .5 + .5; // transform the position of the vertex into UV space
  v_texCoord = 
  quad_pos = QUAD_POS;
}