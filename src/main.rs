use glfw::{Action, Context as _, Key, WindowEvent};
use luminance::backend::texture::Texture as TextureBackend;
use luminance::blending::{Blending, Equation, Factor};
use luminance::context::GraphicsContext;
use luminance::pipeline::{PipelineState, TextureBinding};
use luminance::pixel::{NormRGB8UI, NormUnsigned};
use luminance::render_state::RenderState;
use luminance::shader::Uniform;
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
            main_loop(surface);
        }

        Err(e) => {
            eprintln!("cannot create graphics surface:\n{}", e);
            exit(1);
        }
    }
}

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
}

const TEX_VS: &str = include_str!("texture-vs.glsl");
const TEX_FS: &str = include_str!("texture-fs.glsl");

fn main_loop(mut surface: GlfwSurface) {
    let mut back_buffer = surface.back_buffer().unwrap();

    let img = read_image(Path::new("test_texture.png")).expect("error while reading image on disk");
    let (width, height) = img.dimensions();
    let mut tex = load_from_disk(&mut surface, img);

    let mut tex_program = surface
        .new_shader_program::<(), (), ShaderInterface>()
        .from_strings(TEX_VS, None, None, TEX_FS)
        .expect("program creation")
        .ignore_warnings();
    let tex_tess = surface
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
    let mut resize = false;

    'app: loop {
        surface.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                WindowEvent::FramebufferSize(..) => {
                    resize = true;
                }
                _ => (),
            }
        }

        if resize {
            back_buffer = surface.back_buffer().unwrap();
            resize = false;
        }

        let render = surface
            .new_pipeline_gate()
            .pipeline(
                &back_buffer,
                &PipelineState::default(),
                |pipeline, mut shd_gate| {
                    let bound_tex = pipeline.bind_texture(&mut tex)?;

                    shd_gate.shade(&mut tex_program, |mut iface, uni, mut rdr_gate| {
                        iface.set(&uni.tex, bound_tex.binding());

                        rdr_gate.render(render_st, |mut tess_gate| tess_gate.render(&tex_tess))
                    })
                },
            )
            .assume();

        if render.is_ok() {
            surface.window.swap_buffers();
        } else {
            break 'app;
        }
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
