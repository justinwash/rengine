use glfw::{Action, Context as _, Key, WindowEvent};
use luminance::backend::texture::Texture as TextureBackend;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineState, Pipeline, PipelineError, TextureBinding};
use luminance::pixel::{NormRGB8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::{Program, Uniform};
use luminance::tess::Mode;
use luminance::texture::{Dim2, GenMipmaps, Sampler, Texture};
use luminance::UniformInterface;
use luminance_derive::{Semantics, Vertex};
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::path::Path;
use std::process::exit;
//use std::time::Instant;

fn main() {
    // our graphics surface
    let dim = WindowDim::Windowed {
        width: 1280,
        height: 720,
    };
    let surface = GlfwSurface::new_gl33("Hello, world!", WindowOpt::default().set_dim(dim));

    match surface {
        Ok(surface) => {
            eprintln!("graphics surface created");
            let sprite_renderer = SpriteRenderer::new(surface);
        }

        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }
}

<<<<<<< HEAD
#[derive(Copy, Clone, Debug, Semantics)]
pub enum VertexSemantics {
    #[sem(name = "position", repr = "[f32; 2]", wrapper = "VertexPosition")]
    Position,
    #[sem(name = "color", repr = "[u8; 3]", wrapper = "VertexRGB")]
    Color,
}

#[derive(Copy, Clone, Debug, Vertex)]
#[vertex(sem = "VertexSemantics")]
pub struct Vertex {
    #[allow(dead_code)]
    position: VertexPosition,

    #[allow(dead_code)]
    #[vertex(normalized = "true")]
    color: VertexRGB,
}

const VERTICES: [Vertex; 3] = [
    Vertex::new(
        VertexPosition::new([-0.5, -0.5]),
        VertexRGB::new([255, 0, 0]),
    ),
    Vertex::new(
        VertexPosition::new([0.5, -0.5]),
        VertexRGB::new([0, 255, 0]),
    ),
    Vertex::new(VertexPosition::new([0., 0.5]), VertexRGB::new([0, 0, 255])),
];

#[derive(UniformInterface)]
struct ShaderInterface {
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
=======
use luminance::shader::{Program, Uniform};
use luminance::tess::{Mode, Tess};
use luminance::texture::Dim2;
use luminance_derive::UniformInterface;
use luminance_gl::gl33::GL33;

use crate::assets::{sprite::SpriteAsset, AssetManager, Handle};
use crate::core::colors::RgbaColor;
use crate::core::transform::Transform;
use luminance::shading_gate::ShadingGate;
use serde_derive::{Deserialize, Serialize};
use std::time::Instant;

const VS: &'static str = include_str!("texture-vs.glsl");
const FS: &'static str = include_str!("texture-fs.glsl");

/// Let's make it easy for now...
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Sprite {
    pub id: String,
>>>>>>> d49d090bb5d1298a385d88d493c429c405e90b05
}

/// Attach this component to an entity with a sprite to make it BLINK! KIRA KIRA!
pub struct Blink {
    pub color: [f32; 4],
    pub amplitude: f32,
}

pub struct Tint {
    pub color: RgbaColor,
}

#[derive(UniformInterface)]
pub struct ShaderUniform {
    projection: Uniform<[[f32; 4]; 4]>,
    view: Uniform<[[f32; 4]; 4]>,
    model: Uniform<[[f32; 4]; 4]>,

    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,

    should_blink: Uniform<bool>,
    blink_color: Uniform<[f32; 4]>,
    should_tint: Uniform<bool>,
    tint_color: Uniform<[f32; 4]>,
    time: Uniform<f32>,
    amplitude: Uniform<f32>,
}

pub fn new_shader<B>(surface: GlfwSurface) -> Program<GL33, (), (), ShaderUniform>
where
    B: GraphicsContext<Backend = GL33>,
{
    surface
        .new_shader_program::<(), (), ShaderUniform>()
        .from_strings(VS, None, None, FS)
        .expect("Program creation")
        .ignore_warnings()
}

pub struct SpriteRenderer<S>
where
    S: GraphicsContext<Backend = GL33>,
{
    render_st: RenderState,
    tess: Tess<S::Backend, ()>,

    /// used to send elapsed time to shader.
    creation_time: Instant,

    shader: Program<S::Backend, (), (), ShaderUniform>,
}

impl<S> SpriteRenderer<S>
where
    S: GraphicsContext<Backend = GL33>,
{
    pub fn new(surface: &mut S) -> SpriteRenderer<S> {
        let render_st = RenderState::default()
            .set_depth_test(None)
            .set_blending_separate(
                Blending {
                    equation: Equation::Additive,
                    src: Factor::SrcAlpha,
                    dst: Factor::SrcAlphaComplement,
                },
                Blending {
                    equation: Equation::Additive,
                    src: Factor::One,
                    dst: Factor::Zero,
                },
            );
        let tess = surface
            .new_tess()
            .set_vertex_nb(4)
            .set_mode(Mode::TriangleFan)
            .build()
            .expect("Tess creation");
        SpriteRenderer {
            render_st,
            tess,
            creation_time: Instant::now(),
            shader: new_shader(surface),
        }
    }

    pub fn main_loop(
        &mut self,
        pipeline: &Pipeline<S::Backend>,
        shd_gate: &mut ShadingGate<S::Backend>,
        proj_matrix: &glam::Mat4,
        view: &glam::Mat4,
        world: &hecs::World,
        textures: &mut AssetManager<S, SpriteAsset<S>>,
    ) -> Result<(), PipelineError> {
        let shader = &mut self.shader;
        let render_state = &self.render_st;
        let tess = &self.tess;

        let elapsed = self.creation_time.elapsed().as_secs_f32();

        shd_gate.shade(shader, |mut iface, uni, mut rdr_gate| {
            iface.set(&uni.projection, proj_matrix.to_cols_array_2d());
            iface.set(&uni.view, view.to_cols_array_2d());

            for (e, (sprite, transform)) in world.query::<(&Sprite, &Transform)>().iter() {
                if let Some(tex) = textures.get_mut(&Handle(sprite.id.to_string())) {
                    let mut res = Ok(());
                    tex.execute_mut(|asset| {
                        if let Some(tex) = asset.texture() {
                            // In case there is a blink animation, set up the correct uniforms.
                            if let Ok(blink) = world.get::<Blink>(e) {
                                iface.set(&uni.should_blink, true);
                                iface.set(&uni.blink_color, blink.color);
                                iface.set(&uni.time, elapsed);
                                iface.set(&uni.amplitude, blink.amplitude);
                            } else {
                                iface.set(&uni.should_blink, false);
                            }

                            if let Ok(tint) = world.get::<Tint>(e) {
                                iface.set(&uni.should_tint, true);
                                iface.set(&uni.tint_color, tint.color.to_normalized());
                            } else {
                                iface.set(&uni.should_tint, false);
                            }

                            let bound_tex = pipeline.bind_texture(tex);

                            match bound_tex {
                                Ok(bound_tex) => {
                                    iface.set(&uni.tex, bound_tex.binding());
                                    let model = transform.to_model();
                                    iface.set(&uni.model, model.to_cols_array_2d());

                                    res = rdr_gate.render(render_state, |mut tess_gate| {
                                        tess_gate.render(tess)
                                    });
                                }
                                Err(e) => {
                                    res = Err(e);
                                }
                            }
                        }
                    });

                    res?;
                } else {
                    eprintln!("Texture is not loaded {}", sprite.id);
                    textures.load(sprite.id.to_string());
                }
            }

            Ok(())
        })
    }
}

fn read_image(path: &Path) -> Option<image::RgbImage> {
    image::open(path).map(|img| img.flipv().to_rgb8()).ok()
}

fn load_from_disk<B>(surface: &mut B, img: image::RgbImage) -> Texture<B::Backend, Dim2, NormRGB8UI>
where
    B: GraphicsContext,
    B::Backend: TextureBackend<Dim2, NormRGB8UI>,
{
    let (width, height) = img.dimensions();
    let texels = img.into_raw();

    let mut tex = Texture::new(surface, [width, height], 0, Sampler::default())
        .expect("luminance texture creation");

    tex.upload_raw(GenMipmaps::No, &texels).unwrap();

    tex
}
