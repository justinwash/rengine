use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{Pipeline, PipelineError, TextureBinding};
use luminance::pixel::{NormRGB8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::{Program, Uniform};
use luminance::tess::{Mode, Tess};
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance_derive::UniformInterface;
use luminance_gl::gl33::GL33;
use luminance_glfw::GlfwSurface;
use std::collections::HashMap;
use std::convert::TryInto;
use std::path::Path;
use uuid::Uuid;

use crate::utils::transform::*;

use luminance::shading_gate::ShadingGate;
use std::time::Instant;

const VS: &'static str = include_str!("shaders/sprite-vs.glsl");
const FS: &'static str = include_str!("shaders/sprite-fs.glsl");

#[derive(UniformInterface)]
pub struct ShaderInterface {
    bl: Uniform<[f32; 2]>,
    br: Uniform<[f32; 2]>,
    tr: Uniform<[f32; 2]>,
    tl: Uniform<[f32; 2]>,
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

#[derive(Clone)]
pub struct Sprite {
    pub id: Uuid,
    pub sprite_path: String,
    pub position: Position,
    pub size: Size,
    pub image: image::ImageBuffer<image::Rgb<u8>, std::vec::Vec<u8>>,
}

impl Sprite {
    pub fn new(sprite_path: String, position: Position, size: Size) -> Self {
        let image = read_image(Path::new(&sprite_path)).expect("error while reading image on disk");

        Sprite {
            id: Uuid::new_v4(),
            sprite_path: sprite_path,
            position: position,
            size: size,
            image: image,
        }
    }
}

fn get_gl_coords(pos_x: i32, pos_y: i32, width: i32, height: i32) -> [[f32; 2]; 4] {
    let window_width: i32 = 1280;
    let window_height: i32 = 720;

    let pixel_coords = [
        [pos_x, pos_y + height],
        [pos_x + width, pos_y + height],
        [pos_x + width, pos_y],
        [pos_x, pos_y],
    ];

    let half_screen_width = (window_width as f32 / 2.) / window_width as f32;
    let half_screen_height = (window_height as f32 / 2.) / window_height as f32;

    let gl_coords = pixel_coords.iter().map(|coord| {
        let coord_x = ((coord[0] as f32 / window_width as f32) - half_screen_width) * 2.;
        let coord_y = ((coord[1] as f32 / window_height as f32) - half_screen_height) * -2.;
        [coord_x, coord_y]
    });

    gl_coords.collect::<Vec<[f32; 2]>>().try_into().unwrap()
}

fn read_image(path: &Path) -> Option<image::RgbImage> {
    image::open(path).map(|img| img.flipv().to_rgb8()).ok()
}

pub fn load_from_disk(
    surface: &mut GlfwSurface,
    img: image::RgbImage,
) -> Texture<GL33, Dim2, NormRGB8UI> {
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    let mut tex = Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}

pub fn new_shader(surface: &mut GlfwSurface) -> Program<GL33, (), (), ShaderInterface> {
    surface
        .new_shader_program::<(), (), ShaderInterface>()
        .from_strings(VS, None, None, FS)
        .expect("program creation")
        .ignore_warnings()
}

pub struct SpriteRenderer {
    render_state: RenderState,
    tessellator: Tess<GL33, ()>,
    _creation_time: Instant,
    shader_program: Program<GL33, (), (), ShaderInterface>,
    pub sprites: HashMap<Uuid, Sprite>,
    pub textures: HashMap<
        Uuid,
        luminance::texture::Texture<
            luminance_gl::gl33::GL33,
            luminance::texture::Dim2,
            luminance::pixel::NormRGB8UI,
        >,
    >,
}

impl SpriteRenderer {
    pub fn new(surface: &mut GlfwSurface) -> SpriteRenderer {
        let render_state = RenderState::default().set_blending(Blending {
            equation: Equation::Additive,
            src: Factor::SrcAlpha,
            dst: Factor::Zero,
        });
        let tessellator = surface
            .new_tess()
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .expect("Tess creation");
        SpriteRenderer {
            render_state,
            tessellator,
            _creation_time: Instant::now(),
            shader_program: new_shader(surface),
            sprites: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    #[allow(unused)]
    pub fn load_texture(&mut self, surface: &mut GlfwSurface, sprite: Sprite) {
        self.textures
            .insert(sprite.id, load_from_disk(surface, sprite.image));
        
    }

    pub fn render(
        &mut self,
        pipeline: &Pipeline<GL33>,
        shading_gate: &mut ShadingGate<GL33>,
    ) -> Result<(), PipelineError> {
        let shader_program = &mut self.shader_program;
        let render_state = &self.render_state;
        let tessellator = &self.tessellator;
        let textures = &mut self.textures;
        let sprites = &mut self.sprites;
        shading_gate.shade(shader_program, |mut iface, uni, mut rdr_gate| {
            for sprite in sprites {
                let sprite_transform = get_gl_coords(
                    sprite.1.position.x,
                    sprite.1.position.y,
                    sprite.1.size.w,
                    sprite.1.size.h,
                );

                #[allow(unused_assignments)]
                let mut res = Ok(());
                let mut tex = textures.get_mut(&sprite.0).unwrap();
                let bound_tex = pipeline.bind_texture(&mut tex);

                match bound_tex {
                    Ok(bound_tex) => {
                        iface.set(&uni.bl, sprite_transform[0]);
                        iface.set(&uni.br, sprite_transform[1]);
                        iface.set(&uni.tr, sprite_transform[2]);
                        iface.set(&uni.tl, sprite_transform[3]);
                        iface.set(&uni.tex, bound_tex.binding());
                        res = rdr_gate
                            .render(render_state, |mut tess_gate| tess_gate.render(tessellator));
                    }
                    Err(e) => {
                        res = Err(e);
                    }
                }

                res?;
            }

            Ok(())
        })
    }
}
