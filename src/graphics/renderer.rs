use crate::graphics::sprite::*;
use luminance::context::GraphicsContext;
use luminance::framebuffer::Framebuffer;
use luminance::pipeline::{PipelineError, PipelineState, Render};
use luminance::texture::Dim2;
use luminance_gl::GL33;
use luminance_glfw::GlfwSurface;

pub struct Renderer {
  pub sprites: Vec<Sprite>,
  pub sprite_renderer: SpriteRenderer,
}

impl Renderer {
  pub fn new(surface: &mut GlfwSurface) -> Renderer {
    let sprites = Vec::new();
    let sprite_renderer = SpriteRenderer::new(surface);
    Self {
      sprites,
      sprite_renderer,
    }
  }

  pub fn render(
    &mut self,
    mut surface: &mut GlfwSurface,
    back_buffer: &mut Framebuffer<GL33, Dim2, (), ()>,
  ) -> Render<PipelineError> {
    for sprite in &self.sprites {
      let sprite_cl = sprite.clone();
      self
        .sprite_renderer
        .textures
        .insert(sprite.id, load_from_disk(&mut surface, sprite_cl.image));
    }

    surface
      .new_pipeline_gate()
      .pipeline(
        &back_buffer,
        &PipelineState::default().set_clear_color([0.0, 0.0, 0.0, 1.0]),
        |pipeline, mut shading_gate| {
          self
            .sprite_renderer
            .render(&pipeline, &mut shading_gate, &mut self.sprites)
        },
      )
      .assume()
  }
}
