const SPR_VS: &str = include_str!("graphics/sprite/sprite-vs.glsl");
const SPR_FS: &str = include_str!("graphics/sprite/sprite-fs.glsl");

pub struct Sprite {
    pub mut sprite_path: String,
    pub mut position: {
      x: i32,
      y: i32,
    },
    pub mut size: {
      w: i32,
      h: i32,
    },
    surface: mut GlfwSurface
    shader_program: luminance::shader::Program<luminance_gl::gl33::GL33, (), (), ShaderInterface>
}

impl Sprite {
  fn new(sprite_path: String, 
    position: {x: i32, y: i32}, 
    size: {w: i32, h: i32}, 
    surface: GlfwSurface) -> Self {
      Sprite {
        sprite_path: sprite_path,
        position: position,
        size: size,
        surface: surface,
        shader_program: surface
        .new_shader_program::<(), (), ShaderInterface>()
        .from_strings(SPR_VS, None, None, SPR_FS)
        .expect("program creation")
        .ignore_warnings();
    let spr_tess = surface
        .new_tess()
        .set_vertex_nb(4)
        .set_mode(Mode::TriangleFan)
        .build()
        .unwrap();
    let render_st = &RenderState::default().set_blending(Blending {
        equation: Equation::Additive,
        src: Factor::SrcAlpha,
        dst: Factor::Zero,
    });
      }
  }

  fn render() {

  }

  use std::convert::TryInto;

fn get_gl_coords(pos_x: i32, pos_y: i32, width: i32, height: i32) -> [[f32; 2]; 4] {
  let WINDOW_WIDTH: i32 = 1280;
  let WINDOW_HEIGHT: i32 = 720;

    let pixel_coords = [
        [pos_x, pos_y + height],
        [pos_x + width, pos_y + height],
        [pos_x + width, pos_y],
        [pos_x, pos_y],
    ];

    let half_screen_width = (WINDOW_WIDTH as f32 / 2.) / WINDOW_WIDTH as f32;
    let half_screen_height = (WINDOW_HEIGHT as f32 / 2.) / WINDOW_HEIGHT as f32;

    let gl_coords = pixel_coords.iter().map(|coord| {
        let coord_x = ((coord[0] as f32 / WINDOW_WIDTH as f32) - half_screen_width) * 2.;
        let coord_y = ((coord[1] as f32 / WINDOW_HEIGHT as f32) - half_screen_height) * -2.;
        [coord_x, coord_y]
    });

    gl_coords.collect::<Vec<[f32; 2]>>().try_into().unwrap()
}
}