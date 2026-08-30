//! Scene-authored keyframe animations: clips live in the scene document as an
//! `animations` array (authored in the editor), the engine loads them with the
//! scene, and [`SceneWorld2D`](crate::SceneWorld2D) plays them by sampling the
//! clips against a clock the host owns.
//!
//! The animation is *data*, not a Rust procedure: the screen that wants to move
//! draws an authored scene, calls `play_animation(id, t0)` and then feeds the
//! animation clock it already uses for gameplay (`apply_animations`) each frame.
//! That clock is the host's — a future live-camera rendering of the same clip
//! drives the race's own playback time through the identical sampler, which is
//! the whole reason these clips live here rather than in game code.
//!
//! Authoring rules (one place to learn them): keyframes are sorted by `t` and
//! the easing on a keyframe describes the segment *leaving* it; `offset_x`,
//! `offset_y`, `rotation_deg` and `scale` are deltas over the target's authored
//! transform; `alpha` multiplies the node's current fill alpha (its `ui_color`
//! when it has one, else its leading sprite's tint); 0/1 frames in `visible`
//! act as display on/off. A finished non-looping clip restores every target to
//! the transform, fill and visibility it had when the clip started.

use crate::math::tween::Easing;
use serde::{Deserialize, Serialize};

/// Which property of the target node a track drives.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnimatedProperty {
    /// Canvas-space x delta added to the target's authored position.
    OffsetX,
    /// Canvas-space y delta added to the target's authored position.
    OffsetY,
    /// Degrees of rotation added to the target's authored rotation.
    RotationDeg,
    /// Multiplier over the target's authored scale (1.0 = unchanged).
    Scale,
    /// Multiplier over the target's fill alpha (1.0 = opaque).
    Alpha,
    /// 0 hides the target, 1 shows it.
    Visible,
}

fn default_ease() -> Easing {
    Easing::Linear
}

impl AnimatedProperty {
    pub const ALL: &'static [AnimatedProperty] = &[
        AnimatedProperty::OffsetX,
        AnimatedProperty::OffsetY,
        AnimatedProperty::RotationDeg,
        AnimatedProperty::Scale,
        AnimatedProperty::Alpha,
        AnimatedProperty::Visible,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::OffsetX => "offset_x",
            Self::OffsetY => "offset_y",
            Self::RotationDeg => "rotation_deg",
            Self::Scale => "scale",
            Self::Alpha => "alpha",
            Self::Visible => "visible",
        }
    }
}

/// One sample on a track: where in `t` (clip time, seconds) `value` is held.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AnimKeyframe {
    #[serde(default)]
    pub t: f32,
    #[serde(default)]
    pub value: f32,
    /// How the segment *leaving* this keyframe interpolates to the next.
    #[serde(default = "default_ease")]
    pub ease: Easing,
}

/// One animated property of one named node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneAnimTrack {
    /// The node name the track moves (`SceneWorld2D::find_by_name`).
    pub target: String,
    pub property: AnimatedProperty,
    #[serde(default)]
    pub keyframes: Vec<AnimKeyframe>,
}

/// A playable clip: every track runs on one clock, `0.0..=duration`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneAnimClip {
    pub id: String,
    #[serde(default)]
    pub duration: f32,
    /// Authored as `"loop"` in the scene document (tidy authoring), the Rust
    /// identifier cannot be the keyword.
    #[serde(default, rename = "loop")]
    pub looping: bool,
    #[serde(default)]
    pub tracks: Vec<SceneAnimTrack>,
}

/// Sample a track at `timeline_t` (seconds since clip start), if it has any
/// keyframes. Values before the first keyframe and after the last hold.
pub fn sample_track(track: &SceneAnimTrack, timeline_t: f32) -> Option<f32> {
    sample_keyframes(&track.keyframes, timeline_t)
}

/// Piecewise sample over sorted keyframes. Empty → `None`; a single keyframe
/// holds its value; otherwise the enclosing segment interpolates with the
/// segment-start keyframe's easing and clamps at both ends.
pub fn sample_keyframes(keyframes: &[AnimKeyframe], t: f32) -> Option<f32> {
    let first = keyframes.first()?;
    if t <= first.t {
        return Some(first.value);
    }
    let last = keyframes.last().expect("first() succeeded");
    if t >= last.t {
        return Some(last.value);
    }
    // `keyframes` is assumed sorted by `t`; a linear scan finds the enclosing
    // segment (clips are a handful of frames — `ponytail:` a binary search
    // only pays off past ~16 frames per track).
    for pair in keyframes.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if a.t <= t && t <= b.t {
            let span = (b.t - a.t).max(f32::EPSILON);
            let eased = a.ease.apply((t - a.t) / span);
            return Some(a.value + (b.value - a.value) * eased);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kf(t: f32, value: f32, ease: Easing) -> AnimKeyframe {
        AnimKeyframe { t, value, ease }
    }

    fn track(property: AnimatedProperty, keyframes: Vec<AnimKeyframe>) -> SceneAnimTrack {
        SceneAnimTrack {
            target: "n".into(),
            property,
            keyframes,
        }
    }

    #[test]
    fn empty_track_samples_nothing() {
        assert_eq!(sample_track(&track(AnimatedProperty::Alpha, vec![]), 0.5), None);
    }

    #[test]
    fn single_keyframe_holds_its_value() {
        let t = track(AnimatedProperty::Alpha, vec![kf(0.4, 0.5, Easing::Linear)]);
        assert_eq!(sample_track(&t, 0.0), Some(0.5));
        assert_eq!(sample_track(&t, 0.4), Some(0.5));
        assert_eq!(sample_track(&t, 9.0), Some(0.5));
    }

    #[test]
    fn keyframes_interpolate_linearly_between_frames_and_clamp_outside() {
        let t = track(
            AnimatedProperty::OffsetX,
            vec![kf(0.0, 0.0, Easing::Linear), kf(1.0, 100.0, Easing::Linear)],
        );
        assert_eq!(sample_track(&t, -1.0), Some(0.0));
        assert_eq!(sample_track(&t, 0.0), Some(0.0));
        assert_eq!(sample_track(&t, 0.5).unwrap(), 50.0);
        assert_eq!(sample_track(&t, 1.0), Some(100.0));
        assert_eq!(sample_track(&t, 2.0), Some(100.0));
    }

    #[test]
    fn easing_is_owned_by_the_leaving_keyframe() {
        // OutBack overshoots: at the segment midpoint the value is *past* the
        // destination, proving the easing transformed the raw fraction.
        let t = track(
            AnimatedProperty::OffsetX,
            vec![kf(0.0, 0.0, Easing::OutBack), kf(1.0, 10.0, Easing::Linear)],
        );
        let mid = sample_track(&t, 0.5).unwrap();
        assert!(mid > 5.0, "OutBack overshoot expected, got {mid}");
        // The final keyframe's easing never runs: endpoints clamp.
        let blunt = track(
            AnimatedProperty::OffsetX,
            vec![kf(0.0, 0.0, Easing::Linear), kf(1.0, 10.0, Easing::InExpo)],
        );
        assert_eq!(sample_track(&blunt, 1.0), Some(10.0));
    }

    #[test]
    fn mid_clip_frames_use_the_segment_that_contains_them() {
        let t = track(
            AnimatedProperty::OffsetX,
            vec![
                kf(0.0, 0.0, Easing::Linear),
                kf(0.2, 25.0, Easing::Linear),
                kf(1.0, 50.0, Easing::Linear),
            ],
        );
        assert_eq!(sample_track(&t, 0.1).unwrap(), 12.5);
        assert_eq!(sample_track(&t, 0.6).unwrap(), 37.5); // 25→50 at its midpoint
    }
}