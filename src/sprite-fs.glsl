in vec2 v_uv;
in vec2 tex_uv;
in vec2[4] quad_pos;
out vec4 frag;

uniform sampler2D tex;

void main() {
  frag = texture(tex, tex_uv);
}