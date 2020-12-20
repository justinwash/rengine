uniform float pos_x;
uniform float pos_y;
uniform int width;
uniform int height;
uniform int screen_width;
uniform int screen_height;

out vec2 v_uv;

void main() {
  int half_screen_width = (screen_width / 2) / screen_width;
  int half_screen_height = (screen_height / 2) / screen_height;

  vec2 top_left = vec2((pos_x / screen_width) - half_screen_width, (pos_y / screen_height) - half_screen_height);

  vec2[4] QUAD_POS = vec2[](
  top_left,
  vec2( 1., -1.),
  vec2( 1.,  1.),
  vec2(-1.,  1.)
);

  vec2 p = QUAD_POS[gl_VertexID];

  gl_Position = vec4(p, 0., 1.);
  v_uv = p * .5 + .5; // transform the position of the vertex into UV space
}