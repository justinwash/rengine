pub mod transport;

pub use ggrs;

pub trait InputT:
    Copy
    + Clone
    + PartialEq
    + Default
    + bytemuck::Pod
    + bytemuck::Zeroable
    + serde::Serialize
    + serde::de::DeserializeOwned
    + 'static
{
}
impl<T> InputT for T where
    T: Copy
        + Clone
        + PartialEq
        + Default
        + bytemuck::Pod
        + bytemuck::Zeroable
        + serde::Serialize
        + serde::de::DeserializeOwned
        + 'static
{
}

#[derive(Debug)]
pub struct GgrsConfig<I: InputT>(std::marker::PhantomData<I>);

impl<I: InputT> ggrs::Config for GgrsConfig<I> {
    type Input = I;
    type State = Vec<u8>;
    type Address = String;
}

pub struct OnlineConfig {
    pub local_port: u16,
    pub remote_addr: String,
    pub local_player: usize,
}

pub enum SessionMode {
    Local,
    SyncTest { check_distance: usize },
    Online(OnlineConfig),
}

pub struct RollbackConfig {
    pub num_players: usize,
    pub input_delay: usize,
    pub max_prediction: usize,
    pub fps: u32,
    pub mode: SessionMode,
    pub max_frames: Option<u32>,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            num_players: 2,
            input_delay: 2,
            max_prediction: 8,
            fps: 60,
            mode: SessionMode::Local,
            max_frames: None,
        }
    }
}

pub trait Rollbackable {
    type Input: InputT;
    fn advance(&mut self, inputs: &[Self::Input]);
    fn save(&self) -> Vec<u8>;
    fn load(&mut self, data: &[u8]);
}

enum SessionVariant<I: InputT> {
    Local,
    SyncTest(ggrs::SyncTestSession<GgrsConfig<I>>),
    P2P(ggrs::P2PSession<GgrsConfig<I>>),
}

pub struct RollbackSession<I: InputT> {
    variant: SessionVariant<I>,
    local_player: usize,
    num_players: usize,
    fixed_dt: f32,
    accumulator: f32,
    frame: u32,
    desync_detected: bool,
    max_frames: Option<u32>,
    headless: bool,
}

impl<I: InputT> RollbackSession<I> {
    pub fn new(config: RollbackConfig, headless: bool) -> Result<Self, Box<dyn std::error::Error>> {
        let fixed_dt = 1.0 / config.fps as f32;
        let num_players = config.num_players;
        let max_frames = config.max_frames;
        let mut local_player: usize = 0;

        let variant = match config.mode {
            SessionMode::Local => SessionVariant::Local,

            SessionMode::SyncTest { check_distance } => {
                let mut builder = ggrs::SessionBuilder::<GgrsConfig<I>>::new()
                    .with_num_players(num_players)
                    .with_max_prediction_window(config.max_prediction)
                    .with_input_delay(config.input_delay)
                    .with_check_distance(check_distance);
                for p in 0..num_players {
                    builder = builder.add_player(ggrs::PlayerType::Local, p)?;
                }
                SessionVariant::SyncTest(builder.start_synctest_session()?)
            }

            SessionMode::Online(online_cfg) => {
                let socket = transport::UdpNonBlockingSocket::bind(online_cfg.local_port)?;
                local_player = online_cfg.local_player;
                let remote_player = 1 - local_player;

                let builder = ggrs::SessionBuilder::<GgrsConfig<I>>::new()
                    .with_num_players(num_players)
                    .with_max_prediction_window(config.max_prediction)
                    .with_input_delay(config.input_delay)
                    .with_desync_detection_mode(ggrs::DesyncDetection::On { interval: 1 });
                let builder = builder.add_player(ggrs::PlayerType::Local, local_player)?;
                let builder = builder.add_player(
                    ggrs::PlayerType::Remote(online_cfg.remote_addr),
                    remote_player,
                )?;
                SessionVariant::P2P(builder.start_p2p_session(socket)?)
            }
        };

        Ok(Self {
            variant,
            local_player,
            num_players,
            fixed_dt,
            accumulator: 0.0,
            frame: 0,
            desync_detected: false,
            max_frames,
            headless,
        })
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }
    pub fn local_player(&self) -> usize {
        self.local_player
    }
    pub fn num_players(&self) -> usize {
        self.num_players
    }
    pub fn desync_detected(&self) -> bool {
        self.desync_detected
    }
    pub fn fixed_dt(&self) -> f32 {
        self.fixed_dt
    }

    pub fn confirmed_frame(&self) -> Option<i32> {
        match &self.variant {
            SessionVariant::P2P(sess) => Some(sess.confirmed_frame()),
            _ => None,
        }
    }

    pub fn max_frames_reached(&self) -> bool {
        if let Some(mf) = self.max_frames {
            match &self.variant {
                SessionVariant::P2P(sess) => sess.confirmed_frame() >= mf as i32,
                _ => self.frame >= mf,
            }
        } else {
            false
        }
    }

    pub fn update(
        &mut self,
        dt: f32,
        inputs: &[I],
        sim: &mut impl Rollbackable<Input = I>,
    ) -> bool {
        if !self.headless {
            self.accumulator += dt;
            if self.accumulator < self.fixed_dt {
                if let SessionVariant::P2P(sess) = &mut self.variant {
                    sess.poll_remote_clients();
                }
                return false;
            }
            self.accumulator -= self.fixed_dt;
            if self.accumulator > self.fixed_dt {
                self.accumulator = 0.0;
            }
        }

        match &mut self.variant {
            SessionVariant::Local => {
                sim.advance(inputs);
            }

            SessionVariant::SyncTest(sess) => {
                for (p, &inp) in inputs.iter().enumerate() {
                    sess.add_local_input(p, inp).expect("ggrs: add_local_input");
                }
                match sess.advance_frame() {
                    Ok(requests) => {
                        for req in requests {
                            handle_request(sim, req);
                        }
                    }
                    Err(e) => log::error!("GGRS SyncTest error: {e:?}"),
                }
            }

            SessionVariant::P2P(sess) => {
                sess.poll_remote_clients();

                for ev in sess.events() {
                    match ev {
                        ggrs::GgrsEvent::Synchronized { .. } => {
                            log::info!("GGRS: synchronized with remote");
                        }
                        ggrs::GgrsEvent::Disconnected { .. } => {
                            log::warn!("GGRS: remote disconnected");
                        }
                        ggrs::GgrsEvent::DesyncDetected { .. } => {
                            log::error!("GGRS: DESYNC DETECTED");
                            self.desync_detected = true;
                        }
                        _ => {}
                    }
                }

                if let Err(e) = sess.add_local_input(self.local_player, inputs[self.local_player]) {
                    log::warn!("add_local_input: {e:?}");
                }
                match sess.advance_frame() {
                    Ok(requests) => {
                        for req in requests {
                            handle_request(sim, req);
                        }
                    }
                    Err(ggrs::GgrsError::PredictionThreshold) => {}
                    Err(ggrs::GgrsError::NotSynchronized) => {}
                    Err(e) => log::error!("GGRS P2P error: {e:?}"),
                }
            }
        }

        self.frame += 1;

        if self.headless {
            if let SessionVariant::P2P(_) = &self.variant {
                std::thread::sleep(std::time::Duration::from_secs_f32(self.fixed_dt));
            }
        }

        true
    }
}

fn handle_request<R: Rollbackable>(sim: &mut R, request: ggrs::GgrsRequest<GgrsConfig<R::Input>>) {
    match request {
        ggrs::GgrsRequest::SaveGameState { cell, frame } => {
            let data = sim.save();
            let checksum = fletcher64(&data);
            cell.save(frame, Some(data), Some(checksum as u128));
        }
        ggrs::GgrsRequest::LoadGameState { cell, .. } => {
            let data = cell.load().expect("ggrs: loaded state should be Some");
            sim.load(&data);
        }
        ggrs::GgrsRequest::AdvanceFrame { inputs } => {
            let plain: Vec<R::Input> = inputs.iter().map(|(i, _status)| *i).collect();
            sim.advance(&plain);
        }
    }
}

pub fn fletcher64(data: &[u8]) -> u64 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    for &b in data {
        s1 = s1.wrapping_add(b as u32);
        s2 = s2.wrapping_add(s1);
    }
    ((s2 as u64) << 32) | s1 as u64
}
