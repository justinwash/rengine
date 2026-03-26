mod ai;
mod art;
mod car;
mod driver;
mod input;
mod physics;
mod race;
mod render;
mod state;
mod telemetry;
mod track;
mod track_visuals;

use rengine::*;
use state::RacingGame;

const NUM_CARS: usize = 12;
const RACE_LAPS: u32 = 5;

impl Game for RacingGame {
    fn new(engine: &mut Engine) -> Self {
        let track = track::Track::new();
        let white_tex = engine.white_texture();

        // Generate grid positions
        let grid = track.grid_positions(NUM_CARS);

        // Create cars with unique sprites
        let team_colors: Vec<Color> = (0..NUM_CARS)
            .map(|i| {
                let tmp = car::Car::new(i, Vec2::ZERO, 0.0, white_tex);
                tmp.body_color
            })
            .collect();

        let cars: Vec<car::Car> = (0..NUM_CARS)
            .map(|i| {
                let tex = art::car_sprite(engine, team_colors[i]);
                let (pos, rot) = grid[i];
                car::Car::new(i, pos, rot, tex)
            })
            .collect();

        let race = race::Race::new(RACE_LAPS);
        let visuals = track_visuals::TrackVisuals::new();
        let telemetry = telemetry::Telemetry::new(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

        RacingGame {
            track,
            cars,
            race,
            white_tex,
            visuals,
            telemetry,
            camera_target: 0,
            camera_zoom: 1.0,
            camera_offset: Vec2::ZERO,
        }
    }

    fn update(&mut self, engine: &Engine) {
        input::handle_input(self, engine);
        let dt = engine.dt();
        self.race.update(&mut self.cars, &self.track, dt);
        self.telemetry.log_frame(&self.cars, &self.track);
    }

    fn render(&mut self, engine: &Engine, frame: &mut Frame) {
        render::draw(self, engine, frame);
    }
}

fn main() {
    rengine::run::<RacingGame>(EngineConfig {
        title: "Formula Rogue".into(),
        width: 1280,
        height: 720,
        vsync: true,
        ..Default::default()
    })
    .unwrap();
}
