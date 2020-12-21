in vec2 v_uv;
in vec2[4] quad_pos;
out vec4 frag;

uniform sampler2D tex;

void main() {
  float quad_width = quad_pos[1][0] - quad_pos[0][0];
  float quad_height = quad_pos[2][1] - quad_pos[1][1];

  float half_quad_width = (quad_width / 2.) / quad_width;
  float half_quad_height = (quad_height / 2.) / quad_height;

  float tex_x = ((v_uv[0] / quad_width) - half_quad_width) * 2.;
  float tex_y = ((v_uv[1] / quad_height) - half_quad_height) * 2.;

  vec2 tex_uv = vec2(tex_x, tex_y);
  frag = texture(tex, tex_uv);
}