use rengine::*;

const GAME_W: u32 = 320;
const GAME_H: u32 = 240;

struct PostFxDemo {
    checker_a: TextureId,
    checker_b: TextureId,
    effect_index: usize,
    frame_count: u32,
    demo: bool,
    max_frames: Option<u32>,
}

const CUSTOM_SHADER: &str = r#"
struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VsOut {
    var out: VsOut;
    let x = f32(i32(vi & 1u)) * 4.0 - 1.0;
    let y = f32(i32(vi >> 1u)) * 4.0 - 1.0;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>((x + 1.0) / 2.0, (1.0 - y) / 2.0);
    return out;
}

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;

struct PostFxParams {
    params: array<f32, 8>,
    resolution: vec2<f32>,
    _pad: vec2<f32>,
};
@group(1) @binding(0) var<uniform> u: PostFxParams;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let color = textureSample(t_source, s_source, in.uv);
    let sepia_r = dot(color.rgb, vec3<f32>(0.393, 0.769, 0.189));
    let sepia_g = dot(color.rgb, vec3<f32>(0.349, 0.686, 0.168));
    let sepia_b = dot(color.rgb, vec3<f32>(0.272, 0.534, 0.131));
    return vec4<f32>(sepia_r, sepia_g, sepia_b, color.a);
}
"#;

fn effect_list() -> Vec<(&'static str, PostEffect)> {
    vec![
        ("None", PostEffect::Invert),
        (
            "Vignette",
            PostEffect::Vignette {
                intensity: 0.8,
                radius: 0.6,
                softness: 0.4,
            },
        ),
        ("Blur", PostEffect::Blur { radius: 3.0 }),
        (
            "Bloom",
            PostEffect::Bloom {
                threshold: 0.5,
                intensity: 0.6,
            },
        ),
        (
            "Color Grade",
            PostEffect::ColorGrade {
                brightness: 1.2,
                contrast: 1.3,
                saturation: 0.5,
            },
        ),
        (
            "CRT",
            PostEffect::Crt {
                scanline_intensity: 0.4,
                curvature: 0.15,
            },
        ),
        ("Pixelate", PostEffect::Pixelate { pixel_size: 4.0 }),
        (
            "Chromatic Aberration",
            PostEffect::ChromaticAberration { offset: 0.01 },
        ),
        ("Invert", PostEffect::Invert),
        (
            "Custom (Sepia)",
            PostEffect::Custom {
                wgsl_source: CUSTOM_SHADER.to_string(),
            },
        ),
    ]
}

impl Game for PostFxDemo {
    fn new(engine: &mut Engine) -> Self {
        let args: Vec<String> = std::env::args().collect();
        let demo = args.contains(&"--demo".to_string());
        let max_frames = args
            .windows(2)
            .find(|w| w[0] == "--frames")
            .and_then(|w| w[1].parse().ok());

        Self {
            checker_a: engine.create_color_texture(1, 1, Color::from_rgba8(80, 120, 200, 255)),
            checker_b: engine.create_color_texture(1, 1, Color::from_rgba8(200, 80, 80, 255)),
            effect_index: 0,
            frame_count: 0,
            demo,
            max_frames,
        }
    }

    fn update(&mut self, engine: &Engine) {
        self.frame_count += 1;

        let effects = effect_list();

        if self.demo {
            let switch_interval = 90;
            let idx = (self.frame_count / switch_interval) as usize % effects.len();
            if idx != self.effect_index {
                self.effect_index = idx;
                engine.postfx().clear();
                if self.effect_index > 0 {
                    engine.postfx().push(effects[self.effect_index].1.clone());
                }
            }
        } else {
            if engine.input().is_key_pressed(KeyCode::ArrowRight) {
                self.effect_index = (self.effect_index + 1) % effects.len();
                engine.postfx().clear();
                if self.effect_index > 0 {
                    engine.postfx().push(effects[self.effect_index].1.clone());
                }
            }
            if engine.input().is_key_pressed(KeyCode::ArrowLeft) {
                self.effect_index = if self.effect_index == 0 {
                    effects.len() - 1
                } else {
                    self.effect_index - 1
                };
                engine.postfx().clear();
                if self.effect_index > 0 {
                    engine.postfx().push(effects[self.effect_index].1.clone());
                }
            }
        }
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        frame.clear_color = Color::from_rgba8(30, 30, 40, 255);

        let (gw, gh) = engine.game_size();
        let hw = gw as f32 / 2.0;
        let hh = gh as f32 / 2.0;

        let tile = 16.0;
        let cols = (gw as f32 / tile) as i32 + 1;
        let rows = (gh as f32 / tile) as i32 + 1;
        for row in 0..rows {
            for col in 0..cols {
                let tex = if (row + col) % 2 == 0 {
                    self.checker_a
                } else {
                    self.checker_b
                };
                let x = col as f32 * tile - hw;
                let y = row as f32 * tile - hh;
                frame.draw(tex, Vec2::new(x, y), Vec2::new(tile, tile));
            }
        }

        let white = engine.white_texture();
        frame.draw_colored(
            white,
            Vec2::new(-40.0, -40.0),
            Vec2::new(80.0, 80.0),
            Color::from_rgba8(255, 255, 200, 255),
        );

        let effects = effect_list();
        let name = effects[self.effect_index].0;
        let screen_size = engine.window_size();
        let atlas = engine.font_atlas();
        let canvas = frame.canvas(0);
        canvas.rect(
            -hw + 4.0,
            hh - 4.0 - 22.0,
            300.0,
            22.0,
            Color::from_rgba8(0, 0, 0, 180),
            screen_size,
        );
        let info = format!(
            "PostFx: {} [{}/{}] [Left/Right]",
            name,
            self.effect_index + 1,
            effects.len()
        );
        canvas.text(
            -hw + 8.0,
            hh - 8.0,
            &info,
            14.0,
            Color::WHITE,
            screen_size,
            atlas,
        );
    }

    fn should_exit(&self) -> bool {
        if let Some(max) = self.max_frames {
            self.frame_count >= max
        } else {
            false
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rengine::run::<PostFxDemo>(EngineConfig {
        title: "Post-Processing Demo".into(),
        width: 960,
        height: 720,
        render_width: Some(GAME_W),
        render_height: Some(GAME_H),
        scale_mode: ScaleMode::Letterbox,
        ..Default::default()
    })
}
