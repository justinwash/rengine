/// Driver personality affecting AI behaviour.
/// Ported from Godot DriverProfile.
pub struct DriverProfile {
    pub name: String,
    pub abbreviation: String,

    // Skill parameters (0.0–1.0 ish)
    pub corner_speed_factor: f32,
    pub throttle_aggression: f32,
    pub brake_aggression: f32,
    pub consistency: f32, // lower = more random errors
    pub launch_skill: f32,
    pub overtake_aggression: f32,

    /// How cautiously the driver approaches corners (multiplier on look-ahead
    /// distance): 1.0 = early braker, 0.7 = late braker.
    pub cornering_caution: f32,

    /// Running speed-variation noise.
    speed_variation: f32,
    variation_timer: f32,
}

/// Pre-defined grid of F1-esque driver names + stats.
///  (name, abbr, corner_speed, throttle, brake, consistency, launch, overtake, cornering_caution)
const DRIVER_PRESETS: &[(&str, &str, f32, f32, f32, f32, f32, f32, f32)] = &[
    (
        "Max Vortex",
        "VOR",
        1.04,
        1.05,
        1.10,
        0.92,
        0.85,
        0.90,
        0.85,
    ),
    (
        "Lewis Apex",
        "APX",
        1.03,
        1.02,
        0.95,
        0.96,
        0.90,
        0.85,
        0.95,
    ),
    (
        "Charles Senna",
        "SEN",
        1.05,
        1.08,
        1.05,
        0.85,
        0.80,
        0.95,
        0.78,
    ),
    (
        "Lando Storm",
        "STR",
        1.02,
        1.03,
        1.00,
        0.90,
        0.82,
        0.88,
        0.88,
    ),
    (
        "Carlos Blaze",
        "BLZ",
        1.01,
        1.00,
        0.98,
        0.93,
        0.88,
        0.80,
        0.92,
    ),
    (
        "Oscar Circuit",
        "CIR",
        1.00,
        0.98,
        0.95,
        0.95,
        0.86,
        0.75,
        1.00,
    ),
    (
        "Fernando Nitro",
        "NIT",
        1.02,
        1.04,
        1.02,
        0.88,
        0.75,
        0.92,
        0.82,
    ),
    (
        "George Throttle",
        "THR",
        1.01,
        1.01,
        0.97,
        0.91,
        0.83,
        0.82,
        0.90,
    ),
    (
        "Pierre Gasket",
        "GSK",
        0.99,
        0.97,
        0.93,
        0.89,
        0.78,
        0.78,
        0.96,
    ),
    (
        "Yuki Drift",
        "DFT",
        1.00,
        1.06,
        1.08,
        0.82,
        0.76,
        0.88,
        0.75,
    ),
    (
        "Sebastian Legend",
        "LEG",
        0.98,
        0.95,
        0.92,
        0.94,
        0.80,
        0.70,
        1.02,
    ),
    (
        "Kimi Iceman",
        "ICE",
        0.99,
        0.96,
        0.94,
        0.97,
        0.72,
        0.65,
        0.98,
    ),
    (
        "Daniel Ricco",
        "RIC",
        1.01,
        1.02,
        0.99,
        0.87,
        0.84,
        0.86,
        0.86,
    ),
    (
        "Nico Boost",
        "BST",
        1.00,
        0.99,
        0.96,
        0.91,
        0.79,
        0.77,
        0.93,
    ),
    (
        "Valtteri Grid",
        "GRD",
        0.98,
        0.97,
        0.93,
        0.93,
        0.90,
        0.60,
        1.00,
    ),
    (
        "Kevin Magnate",
        "MAG",
        0.97,
        1.03,
        1.05,
        0.80,
        0.74,
        0.93,
        0.76,
    ),
    (
        "Alex Redline",
        "RED",
        0.99,
        1.00,
        0.97,
        0.88,
        0.81,
        0.79,
        0.90,
    ),
    (
        "Mick Junior",
        "JNR",
        0.96,
        0.94,
        0.91,
        0.86,
        0.77,
        0.72,
        1.04,
    ),
    (
        "Zhou Guanyu",
        "ZHO",
        0.97,
        0.96,
        0.93,
        0.87,
        0.78,
        0.74,
        0.97,
    ),
    (
        "Logan Draft",
        "DRA",
        0.95,
        0.93,
        0.90,
        0.84,
        0.73,
        0.70,
        1.05,
    ),
];

impl DriverProfile {
    pub fn preset(index: usize) -> Self {
        let i = index % DRIVER_PRESETS.len();
        let p = DRIVER_PRESETS[i];
        Self {
            name: p.0.to_string(),
            abbreviation: p.1.to_string(),
            corner_speed_factor: p.2,
            throttle_aggression: p.3,
            brake_aggression: p.4,
            consistency: p.5,
            launch_skill: p.6,
            overtake_aggression: p.7,
            cornering_caution: p.8,
            speed_variation: 0.0,
            variation_timer: 0.0,
        }
    }

    /// Get a small random speed variation (simulates human inconsistency).
    /// Call once per frame.
    pub fn update_variation(&mut self, dt: f32) {
        self.variation_timer += dt;
        // Use sin-based noise for deterministic smooth variation
        let freq = 0.7 + (1.0 - self.consistency) * 2.0;
        let amp = (1.0 - self.consistency) * 0.04;
        self.speed_variation = (self.variation_timer * freq).sin() * amp;
    }

    /// Multiplier for target speed including consistency variation.
    pub fn speed_multiplier(&self) -> f32 {
        self.corner_speed_factor + self.speed_variation
    }
}
