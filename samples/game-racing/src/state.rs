use rengine::TextureId;

use crate::car::Car;
use crate::race::Race;
use crate::telemetry::Telemetry;
use crate::track::Track;
use crate::track_visuals::TrackVisuals;

pub struct RacingGame {
    pub track: Track,
    pub cars: Vec<Car>,
    pub race: Race,
    pub white_tex: TextureId,
    pub visuals: TrackVisuals,
    pub telemetry: Telemetry,
    pub camera_target: usize, // index of car being followed
    pub camera_zoom: f32,
    pub camera_offset: rengine::Vec2, // manual pan offset
}
