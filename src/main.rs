use glfw::{Action, Context as _, Key, Window, WindowEvent};
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
use luminance_glfw::GlfwSurface;
use luminance_windowing::{WindowDim, WindowOpt};
use std::path::Path;
use std::process::exit;
//use std::time::Instant;
use crate::sprite::*;

const WINDOW_WIDTH: i32 = 1280;
const WINDOW_HEIGHT: i32 = 720;

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

#[derive(UniformInterface)]
struct ShaderInterface {
    bl: Uniform<[f32; 2]>,
    br: Uniform<[f32; 2]>,
    tr: Uniform<[f32; 2]>,
    tl: Uniform<[f32; 2]>,
    tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
}

const SPR_VS: &str = include_str!("graphics/sprite/sprite-vs.glsl");
const SPR_FS: &str = include_str!("graphics/sprite/sprite-fs.glsl");

fn main_loop(mut surface: GlfwSurface) {
    let back_buffer = surface.back_buffer().unwrap();

    let img = read_image(Path::new("test_texture.png")).expect("error while reading image on disk");
    let (width, height) = img.dimensions();
    let mut tex = load_from_disk(&mut surface, img);

    let mut spr_program = surface
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

    let mut pos_x = 0;
    let mut pos_y = 0;

    'app: loop {
        surface.window.glfw.poll_events();
        for (_, event) in surface.events_rx.try_iter() {
            match event {
                WindowEvent::Close | WindowEvent::Key(Key::Escape, _, Action::Release, _) => {
                    break 'app
                }
                _ => (),
            }
        }

        if Window::get_key(&surface.window, Key::A) == Action::Press
            || Window::get_key(&surface.window, Key::A) == Action::Repeat
        {
            pos_x -= 1;
        }

        if Window::get_key(&surface.window, Key::D) == Action::Press
            || Window::get_key(&surface.window, Key::D) == Action::Repeat
        {
            pos_x += 1;
        }

        if Window::get_key(&surface.window, Key::W) == Action::Press
            || Window::get_key(&surface.window, Key::W) == Action::Repeat
        {
            pos_y -= 1;
        }

        if Window::get_key(&surface.window, Key::S) == Action::Press
            || Window::get_key(&surface.window, Key::S) == Action::Repeat
        {
            pos_y += 1
        }

        let transform = get_gl_coords(
            pos_x,
            pos_y,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
        );

        let sprite = Sprite::new(
            "test_texture.png".to_string(),
            Position { x: pos_x, y: pos_y },
            Size { w: 512, h: 512 },
            &mut surface,
        );

        let render = sprite.render();

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

use std::convert::TryInto;

fn get_gl_coords(pos_x: i32, pos_y: i32, width: i32, height: i32) -> [[f32; 2]; 4] {
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

mod sprite {
    use glfw::{Action, Context as _, Key, Window, WindowEvent};
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
    use luminance_glfw::GlfwSurface;
    use luminance_windowing::{WindowDim, WindowOpt};
    use std::path::Path;
    use std::process::exit;

    use std::convert::TryInto;
    const SPR_VS: &str = include_str!("graphics/sprite/sprite-vs.glsl");
    const SPR_FS: &str = include_str!("graphics/sprite/sprite-fs.glsl");

    #[derive(UniformInterface)]
    pub struct ShaderInterface {
        bl: Uniform<[f32; 2]>,
        br: Uniform<[f32; 2]>,
        tr: Uniform<[f32; 2]>,
        tl: Uniform<[f32; 2]>,
        tex: Uniform<TextureBinding<Dim2, NormUnsigned>>,
    }

    pub struct Position {
        x: i32,
        y: i32,
    }

    pub struct Size {
        w: i32,
        h: i32,
    }

    pub struct Sprite {
        pub sprite_path: String,
        pub position: Position,
        pub size: Size,
        surface: &'static mut GlfwSurface,
        texture: luminance::texture::Texture<
            luminance_gl::gl33::GL33,
            luminance::texture::Dim2,
            luminance::pixel::NormRGB8UI,
        >,
    }

    impl Sprite {
        pub fn new(
            sprite_path: String,
            position: Position,
            size: Size,
            mut surface: &'static mut GlfwSurface,
        ) -> Self {
            let img =
                read_image(Path::new(&sprite_path)).expect("error while reading image on disk");
            let (width, height) = img.dimensions();
            let tex = load_from_disk(surface, img);

            Sprite {
                sprite_path: sprite_path,
                position: position,
                size: size,
                surface: surface,
                texture: tex,
            }
        }

        pub fn render(mut self) -> luminance::pipeline::Render<luminance::pipeline::PipelineError> {
            let mut shader_program = self
                .surface
                .new_shader_program::<(), (), ShaderInterface>()
                .from_strings(SPR_VS, None, None, SPR_FS)
                .expect("program creation")
                .ignore_warnings();
            let tessellator = self
                .surface
                .new_tess()
                .set_vertex_nb(4)
                .set_mode(Mode::TriangleFan)
                .build()
                .unwrap();
            let render_state = &RenderState::default().set_blending(Blending {
                equation: Equation::Additive,
                src: Factor::SrcAlpha,
                dst: Factor::Zero,
            });

            let transform = get_gl_coords(
                self.position.x,
                self.position.y,
                self.size.w.try_into().unwrap(),
                self.size.h.try_into().unwrap(),
            );

            let back_buffer = &self.surface.back_buffer().unwrap();
            let mut texture = self.texture;

            self.surface
                .new_pipeline_gate()
                .pipeline(
                    back_buffer,
                    &PipelineState::default(),
                    |pipeline, mut shd_gate| {
                        let bound_tex = pipeline.bind_texture(&mut texture)?;

                        shd_gate.shade(&mut shader_program, |mut iface, uni, mut rdr_gate| {
                            iface.set(&uni.bl, transform[0]);
                            iface.set(&uni.br, transform[1]);
                            iface.set(&uni.tr, transform[2]);
                            iface.set(&uni.tl, transform[3]);
                            iface.set(&uni.tex, bound_tex.binding());
                            rdr_gate.render(render_state, |mut tess_gate| {
                                tess_gate.render(&tessellator)
                            })
                        })
                    },
                )
                .assume()
        }
    }

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

    fn read_image(path: &Path) -> Option<image::RgbImage> {
        image::open(path).map(|img| img.flipv().to_rgb8()).ok()
    }

    fn load_from_disk<B>(
        surface: &mut B,
        img: image::RgbImage,
    ) -> Texture<B::Backend, Dim2, NormRGB8UI>
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
}
