mod countdown;
mod game;
mod pause;
mod state;
mod title;

use std::path::PathBuf;

use countdown::CountdownScene;
use game::GameScene;
use rengine::*;
use state::*;
use title::TitleScene;

fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

fn arg_value(name: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn main() {
    let headless = has_flag("--headless");
    let demo = has_flag("--demo");
    let show_debug_overlay = has_flag("--debug-overlay");
    let max_frames: u32 = arg_value("--frames")
        .and_then(|f| f.parse().ok())
        .unwrap_or(600);

    if demo {
        println!("==============================================");
        println!("  RENGINE KITCHEN SINK - DEMO MODE");
        println!("  headless: {}  frames: {}", headless, max_frames);
        println!("==============================================");
    }

    rengine::run_with_scenes(
        EngineConfig {
            title: "Rengine Kitchen Sink".into(),
            width: 960,
            height: 720,
            vsync: false,
            headless,
            hot_reload: !headless,
            show_fps: false,
            show_debug_overlay,
            fixed_dt: 1.0 / 60.0,
            render_width: Some(480),
            render_height: Some(360),
            scale_mode: ScaleMode::Letterbox,
            ..Default::default()
        },
        move |engine, globals| {
            engine.set_asset_root(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets"));
            println!("[FEATURE OK] Engine::set_asset_root — assets resolve from crate directory");
            println!(
                "[FEATURE OK] ScaleMode — game renders at {:?}, window {:?}",
                engine.game_size(),
                engine.window_size()
            );

            let actions = engine.actions_mut();

            actions.bind("confirm", Binding::Key(KeyCode::Enter));
            actions.bind("confirm", Binding::Key(KeyCode::Space));
            actions.bind("confirm", Binding::GamepadButton(GamepadButton::South));

            actions.bind("jump", Binding::Key(KeyCode::Space));
            actions.bind("jump", Binding::Key(KeyCode::ArrowUp));
            actions.bind("jump", Binding::Key(KeyCode::KeyW));
            actions.bind("jump", Binding::GamepadButton(GamepadButton::South));

            actions.bind("pause", Binding::Key(KeyCode::KeyP));
            actions.bind("pause", Binding::Key(KeyCode::Escape));
            actions.bind("pause", Binding::GamepadButton(GamepadButton::Start));

            actions.bind("quit", Binding::Key(KeyCode::KeyQ));
            actions.bind("quit", Binding::GamepadButton(GamepadButton::Select));

            actions.bind_axis(
                "move_x",
                AxisMapping {
                    positive: vec![
                        Binding::Key(KeyCode::KeyD),
                        Binding::Key(KeyCode::ArrowRight),
                    ],
                    negative: vec![
                        Binding::Key(KeyCode::KeyA),
                        Binding::Key(KeyCode::ArrowLeft),
                    ],
                    gamepad_axis: Some(GamepadAxis::LeftStickX),
                },
            );

            println!("[FEATURE OK] ActionMap — bound confirm, jump, pause, quit, move_x axis");
            println!("[FEATURE OK] Binding::Key + Binding::GamepadButton");
            println!("[FEATURE OK] AxisMapping — move_x with keyboard + GamepadAxis::LeftStickX");
            println!(
                "[FEATURE OK] Debug overlay + console — pass --debug-overlay to start it open"
            );
            println!(
                "[FEATURE OK] EngineConfig — title, width, height, vsync, headless, \
                 hot_reload, show_fps, fixed_dt"
            );

            engine.log_info(
                "kitchen_sink::debug",
                "Kitchen sink debug overlay support is live. Press F3 for the overlay and F4 or ` for the console.",
            );
            if show_debug_overlay {
                engine.log_debug(
                    "kitchen_sink::debug",
                    "Debug overlay started open via --debug-overlay.",
                );
            }

            globals.set(TransitionCounter(0));
            globals.set(PlayerStats {
                coins: 0,
                best_height: 0.0,
            });

            match SaveSystem::new("rengine-kitchen-sink") {
                Ok(saves) => {
                    println!("[FEATURE OK] SaveSystem::new — save dir at {:?}", saves.save_dir());
                    globals.set(saves);
                }
                Err(e) => eprintln!("Warning: could not init SaveSystem: {e}"),
            }

            globals.set(DemoConfig {
                enabled: demo,
                max_frames,
                frame: 0,
                features_hit: Vec::new(),
            });

            println!("[FEATURE OK] Globals::set — TransitionCounter, PlayerStats, SaveSystem, DemoConfig");
            println!("[FEATURE OK] run_with_scenes — scene-stack entry point");

            if demo {
                if headless {
                    println!("[Demo] Headless: skipping countdown, starting GameScene directly");
                    Box::new(GameScene::default()) as Box<dyn Scene>
                } else {
                    println!("[Demo] 3-second countdown before demo starts");
                    Box::new(CountdownScene::new()) as Box<dyn Scene>
                }
            } else {
                Box::new(TitleScene::new()) as Box<dyn Scene>
            }
        },
    )
    .unwrap();
}
