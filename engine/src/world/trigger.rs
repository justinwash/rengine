use std::collections::{BTreeMap, HashSet};

use crate::math::rect::Rect;
use crate::world::physics::CollisionLayer;

pub type TriggerZoneId = usize;

pub type BodyId = u64;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlapEvent {
    Enter,
    Stay,
    Exit,
}

pub struct TriggerSystem {
    zones: Vec<TriggerZone>,
    prev_overlaps: Vec<HashSet<BodyId>>,
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

    pub fn add_zone(&mut self, zone: TriggerZone) -> TriggerZoneId {
        let id = self.zones.len();
        self.zones.push(zone);
        self.prev_overlaps.push(HashSet::new());
        id
    }

    pub fn zone(&self, id: TriggerZoneId) -> &TriggerZone {
        &self.zones[id]
    }

    pub fn zone_mut(&mut self, id: TriggerZoneId) -> &mut TriggerZone {
        &mut self.zones[id]
    }

    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    pub fn tick(&mut self, bodies: &[(BodyId, Rect, CollisionLayer)]) {
        self.current_events.clear();

        for (zone_id, zone) in self.zones.iter().enumerate() {
            if !zone.enabled {
                for &body_id in &self.prev_overlaps[zone_id] {
                    self.current_events
                        .insert((zone_id, body_id), OverlapEvent::Exit);
                }
                self.prev_overlaps[zone_id].clear();
                continue;
            }

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

            for &body_id in &prev {
                if !self.prev_overlaps[zone_id].contains(&body_id) {
                    self.current_events
                        .insert((zone_id, body_id), OverlapEvent::Exit);
                }
            }
        }
    }

    pub fn events(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId, OverlapEvent)> + '_ {
        self.current_events
            .iter()
            .map(|(&(zone_id, body_id), &event)| (zone_id, body_id, event))
    }

    pub fn entered(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId)> + '_ {
        self.current_events
            .iter()
            .filter(|(_, &event)| event == OverlapEvent::Enter)
            .map(|(&(zone_id, body_id), _)| (zone_id, body_id))
    }

    pub fn exited(&self) -> impl Iterator<Item = (TriggerZoneId, BodyId)> + '_ {
        self.current_events
            .iter()
            .filter(|(_, &event)| event == OverlapEvent::Exit)
            .map(|(&(zone_id, body_id), _)| (zone_id, body_id))
    }

    pub fn overlapping(&self, zone_id: TriggerZoneId, body_id: BodyId) -> bool {
        self.prev_overlaps[zone_id].contains(&body_id)
    }

    pub fn event_for(&self, zone_id: TriggerZoneId, body_id: BodyId) -> Option<OverlapEvent> {
        self.current_events.get(&(zone_id, body_id)).copied()
    }
}

impl Default for TriggerSystem {
    fn default() -> Self {
        Self::new()
    }
}
