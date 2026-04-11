use std::collections::{BTreeMap, HashSet};

use crate::math::rect::Rect;
use crate::world::physics::CollisionLayer;

/// Identifies a trigger zone within a [`TriggerSystem`].
pub type TriggerZoneId = usize;

/// Identifies a body being tracked against trigger zones.
pub type BodyId = u64;

/// A spatial region that detects overlap with tracked bodies.
#[derive(Debug, Clone)]
pub struct TriggerZone {
    pub rect: Rect,
    pub layer: CollisionLayer,
    pub enabled: bool,
}

impl TriggerZone {
    pub fn new(rect: Rect) -> Self {
        Self {
            rect,
            layer: CollisionLayer::default(),
            enabled: true,
        }
    }

    pub fn with_layer(mut self, layer: CollisionLayer) -> Self {
        self.layer = layer;
        self
    }
}

/// An overlap event produced by [`TriggerSystem::tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapEvent {
    /// A body entered a trigger zone this tick.
    Enter,
    /// A body remains inside a trigger zone.
    Stay,
    /// A body left a trigger zone this tick.
    Exit,
}

/// Tracks bodies against trigger zones and produces enter/stay/exit events.
///
/// # Usage
/// 1. Add zones with [`add_zone`].
/// 2. Each tick, call [`tick`] with the current body positions and layers.
/// 3. Query results with [`events`], [`entered`], [`exited`], or [`overlapping`].
pub struct TriggerSystem {
    zones: Vec<TriggerZone>,
    /// For each zone, the set of body IDs that were overlapping last tick.
    prev_overlaps: Vec<HashSet<BodyId>>,
    /// Events produced by the most recent tick, deterministically ordered.
    current_events: BTreeMap<(TriggerZoneId, BodyId), OverlapEvent>,
}

impl TriggerSystem {
    pub fn new() -> Self {
        Self {
            zones: Vec::new(),
            prev_overlaps: Vec::new(),
            current_events: BTreeMap::new(),
        }
    }

    /// Add a trigger zone and return its ID.
    pub fn add_zone(&mut self, zone: TriggerZone) -> TriggerZoneId {
        let id = self.zones.len();
        self.zones.push(zone);
        self.prev_overlaps.push(HashSet::new());
        id
    }

    /// Get a reference to a zone by ID.
    pub fn zone(&self, id: TriggerZoneId) -> &TriggerZone {
        &self.zones[id]
    }

    /// Get a mutable reference to a zone by ID.
    pub fn zone_mut(&mut self, id: TriggerZoneId) -> &mut TriggerZone {
        &mut self.zones[id]
    }

    /// Number of registered zones.
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Update overlap state. Call once per tick with all active bodies.
    ///
    /// Each body is identified by a [`BodyId`], has a spatial [`Rect`], and a
    /// [`CollisionLayer`]. The system compares current overlaps against the
    /// previous tick to produce enter/stay/exit events.
    pub fn tick(&mut self, bodies: &[(BodyId, Rect, CollisionLayer)]) {
        self.current_events.clear();

        for (zone_id, zone) in self.zones.iter().enumerate() {
            if !zone.enabled {
                // Disabled zones produce Exit for any still-tracked bodies,
                // then clear tracking.
                for &body_id in &self.prev_overlaps[zone_id] {
                    self.current_events
                        .insert((zone_id, body_id), OverlapEvent::Exit);
                }
                self.prev_overlaps[zone_id].clear();
                continue;
            }

            // Swap prev_overlaps out, keeping capacity for reuse after clearing.
            let prev: Vec<BodyId> = self.prev_overlaps[zone_id].drain().collect();

            for &(body_id, ref body_rect, ref body_layer) in bodies {
                if !zone.layer.interacts_with(body_layer) {
                    continue;
                }
                if zone.rect.overlaps(body_rect) {
                    let event = if prev.contains(&body_id) {
                        OverlapEvent::Stay
                    } else {
                        OverlapEvent::Enter
                    };
                    self.current_events.insert((zone_id, body_id), event);
                    self.prev_overlaps[zone_id].insert(body_id);
                }
            }

            // Remaining in prev are exits
            for &body_id in &prev {
                if !self.prev_overlaps[zone_id].contains(&body_id) {
                    self.current_events
                        .insert((zone_id, body_id), OverlapEvent::Exit);
                }
            }
        }
    }

    /// Iterate all events from the most recent tick.
    pub fn events(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId, OverlapEvent)> + '_ {
        self.current_events
            .iter()
            .map(|(&(zone_id, body_id), &event)| (zone_id, body_id, event))
    }

    /// Iterate enter events from the most recent tick.
    pub fn entered(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId)> + '_ {
        self.current_events
            .iter()
            .filter(|(_, &event)| event == OverlapEvent::Enter)
            .map(|(&(zone_id, body_id), _)| (zone_id, body_id))
    }

    /// Iterate exit events from the most recent tick.
    pub fn exited(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId)> + '_ {
        self.current_events
            .iter()
            .filter(|(_, &event)| event == OverlapEvent::Exit)
            .map(|(&(zone_id, body_id), _)| (zone_id, body_id))
    }

    /// Check if a specific body is currently overlapping a specific zone.
    pub fn overlapping(&self, zone_id: TriggerZoneId, body_id: BodyId) -> bool {
        self.prev_overlaps[zone_id].contains(&body_id)
    }

    /// Get the event for a specific zone+body pair this tick, if any.
    pub fn event_for(&self, zone_id: TriggerZoneId, body_id: BodyId) -> Option<OverlapEvent> {
        self.current_events.get(&(zone_id, body_id)).copied()
    }
}

impl Default for TriggerSystem {
    fn default() -> Self {
        Self::new()
    }
}
