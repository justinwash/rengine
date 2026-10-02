//! Reading a frame's UI audit (`audit.rs`) for what does not fit.
//!
//! The records say what was drawn; this says what a player would call "text
//! out of its box". Each [`Finding`] is one of:
//!
//! - `offscreen`: text, or a painted box, past the edge of the screen;
//! - `spill`: text ink outside the painted box it sits in (the nearest
//!   ancestor, or the node itself, whose fills cover its rect edge to edge);
//! - `overlap`: two runs of text over each other with nothing opaque between;
//! - `clipped`: text cut by a clip region;
//! - `collide`: text touching or under a sibling box in the same panel;
//! - `stacked`: two painted boxes in the same panel over each other;
//! - `box`: a painted box hanging out of the painted box around it;
//! - `crowded`: two runs on one line closer than a third of an em, so they read
//!   as one;
//! - `dropped`: characters the face could not draw.
//!
//! What a game draws over itself on purpose (a modal, a label on a map, a line
//! it clips and shows whole in a tooltip) is its call, and it says so in
//! [`AuditRules`].

use std::collections::{HashMap, HashSet};

use super::audit::AuditRecord;
use super::Canvas;

/// Pixels of slack before an excursion counts: glyph quads carry about a pixel
/// of antialiasing margin.
const SLACK: f32 = 1.5;
/// A fill this opaque hides what is under it.
const OPAQUE: f32 = 0.9;
/// A painted box's fills reach each edge of its rect to within this.
const BOX_EDGE: f32 = 2.0;
/// Overlaps smaller than this many square pixels are kerning noise.
const MIN_OVERLAP_AREA: f32 = 6.0;
/// Two runs on a line closer than this many ems (of the smaller) read as one.
const CROWD_EMS: f32 = 0.35;
/// Text nearer a sibling box than this touches it.
const COLLIDE_GAP: f32 = 2.0;
/// Two boxes must overlap by this much both ways to be stacked: a marker line
/// across a strip is narrower, by design.
const STACK_MIN: f32 = 4.0;

/// What a game draws over itself on purpose, by node name.
#[derive(Clone, Debug, Default)]
pub struct AuditRules {
    /// Drawn over whatever is there: modals, hover cards. One may cover
    /// anything; text and boxes beside or under it are not its business.
    pub popups: Vec<String>,
    /// Text that belongs to the world, not the UI (a car's label on a map), and
    /// runs under the HUD by design.
    pub world_labels: Vec<String>,
    /// Boxes that are world furniture drawn under the HUD (a gantry standing on
    /// the track): a panel may sit over one.
    pub world_boxes: Vec<String>,
    /// Lines clipped by design, whose whole text the game shows another way
    /// (a hover tooltip).
    pub clip_ok: Vec<String>,
}

/// One thing that does not fit. Rects are `[x, y, w, h]` in screen pixels.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Finding {
    pub kind: &'static str,
    /// The node, as its ancestors' names from the root (`<immediate>` for text
    /// drawn outside a scene).
    pub path: String,
    pub text: String,
    /// Pixels past the edge; for `overlap`, the area in square pixels.
    pub amount: f32,
    pub ink: Option<[f32; 4]>,
    pub other_rect: Option<[f32; 4]>,
    /// The other node in a pair, or the box something spilled out of.
    pub other: String,
    pub detail: String,
}

impl Finding {
    /// One line for a log: `spill 7.0 root/panel/label 'TEXT' in root/panel`.
    pub fn line(&self) -> String {
        let mut s = format!("{} {:.1} {}", self.kind, self.amount, self.path);
        if !self.text.is_empty() {
            s.push_str(&format!(" '{}'", self.text));
        }
        if !self.other.is_empty() {
            s.push_str(&format!(" | {}", self.other));
        }
        if !self.detail.is_empty() {
            s.push_str(&format!(" ({})", self.detail));
        }
        s
    }
}

type Rect = [f32; 4];
type Key = (usize, u32); // (layer, node id)

fn inter(a: Rect, b: Rect) -> Option<Rect> {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1 - x0, y1 - y0])
}

/// How far `inner` reaches past `outer` on its worst side, and which side.
fn excursion(inner: Rect, outer: Rect) -> (f32, &'static str) {
    [
        (outer[0] - inner[0], "left"),
        (outer[1] - inner[1], "top"),
        ((inner[0] + inner[2]) - (outer[0] + outer[2]), "right"),
        ((inner[1] + inner[3]) - (outer[1] + outer[3]), "bottom"),
    ]
    .into_iter()
    .fold((f32::MIN, "left"), |best, s| if s.0 > best.0 { s } else { best })
}

fn contains(outer: Rect, inner: Rect) -> bool {
    excursion(inner, outer).0 <= 0.01
}

struct Node<'a> {
    parent: Option<u32>,
    name: &'a str,
    rect: Option<Rect>,
}

struct Text<'a> {
    seq: usize,
    layer: usize,
    node: Option<u32>,
    text: &'a str,
    size: f32,
    ink: Rect,
    clip: Option<Rect>,
    dropped: u32,
}

struct Fill {
    seq: usize,
    rect: Rect,
    alpha: f32,
    clip: Option<Rect>,
}

struct Frame<'a> {
    screen: Rect,
    rules: &'a AuditRules,
    nodes: HashMap<Key, Node<'a>>,
    texts: Vec<Text<'a>>,
    fills: Vec<Fill>,
    /// Each node's own fills, and when it first painted.
    node_fills: HashMap<Key, Vec<Rect>>,
    /// The clip a node's fills drew under, if any: a row of a scrolled list
    /// is only as big as the part of it the viewport shows.
    node_clip: HashMap<Key, Rect>,
    node_seq: HashMap<Key, usize>,
}

impl<'a> Frame<'a> {
    fn new(screen: (u32, u32), canvases: &'a [Canvas], rules: &'a AuditRules) -> Self {
        let mut frame = Frame {
            screen: [0.0, 0.0, screen.0 as f32, screen.1 as f32],
            rules,
            nodes: HashMap::new(),
            texts: Vec::new(),
            fills: Vec::new(),
            node_fills: HashMap::new(),
            node_clip: HashMap::new(),
            node_seq: HashMap::new(),
        };
        let mut seq = 0;
        for (layer, canvas) in canvases.iter().enumerate() {
            let Some(audit) = canvas.audit() else {
                continue;
            };
            for record in &audit.records {
                seq += 1;
                match record {
                    AuditRecord::Node {
                        id,
                        parent,
                        name,
                        rect,
                        ..
                    } => {
                        frame.nodes.insert(
                            (layer, *id),
                            Node {
                                parent: *parent,
                                name,
                                rect: *rect,
                            },
                        );
                    }
                    AuditRecord::Text {
                        node,
                        text,
                        size,
                        ink: Some(ink),
                        clip,
                        dropped,
                    } if !text.trim().is_empty() => frame.texts.push(Text {
                        seq,
                        layer,
                        node: *node,
                        text,
                        size: *size,
                        ink: *ink,
                        clip: *clip,
                        dropped: *dropped,
                    }),
                    AuditRecord::Fill {
                        node,
                        rect,
                        alpha,
                        clip,
                    } => {
                        if let (Some(node), true) = (node, *alpha > 0.05) {
                            frame.node_fills.entry((layer, *node)).or_default().push(*rect);
                            frame.node_seq.entry((layer, *node)).or_insert(seq);
                            if let Some(clip) = clip {
                                frame.node_clip.entry((layer, *node)).or_insert(*clip);
                            }
                        }
                        frame.fills.push(Fill {
                            seq,
                            rect: *rect,
                            alpha: *alpha,
                            clip: *clip,
                        });
                    }
                    _ => {}
                }
            }
        }
        frame
    }

    fn name(&self, layer: usize, id: u32) -> &str {
        self.nodes.get(&(layer, id)).map_or("", |n| n.name)
    }

    fn ancestors(&self, layer: usize, id: Option<u32>) -> Vec<u32> {
        let mut out = Vec::new();
        let mut cur = id;
        while let Some(c) = cur {
            out.push(c);
            cur = self.nodes.get(&(layer, c)).and_then(|n| n.parent);
        }
        out
    }

    fn path(&self, layer: usize, id: Option<u32>) -> String {
        if id.is_none() {
            return "<immediate>".to_string();
        }
        let names: Vec<&str> = self
            .ancestors(layer, id)
            .into_iter()
            .rev()
            .map(|c| self.name(layer, c))
            .filter(|n| !n.is_empty())
            .collect();
        names.join("/")
    }

    fn is(&self, list: &[String], name: &str) -> bool {
        list.iter().any(|n| n == name)
    }

    fn in_popup(&self, layer: usize, id: Option<u32>) -> bool {
        self.ancestors(layer, id)
            .into_iter()
            .any(|c| self.is(&self.rules.popups, self.name(layer, c)))
    }

    /// What shows of a painted box: its rect, cut to the clip it drew under.
    /// `None` for a box scrolled wholly out of its viewport.
    fn shown(&self, key: Key) -> Option<Rect> {
        let rect = self.painted(key)?;
        match self.node_clip.get(&key) {
            Some(&clip) => inter(rect, clip),
            None => Some(rect),
        }
    }

    /// The node's rect, if its own fills cover it edge to edge: a background,
    /// a border, a sprite panel. A layout slot paints nothing and is not a box.
    fn painted(&self, key: Key) -> Option<Rect> {
        let rect = self.nodes.get(&key)?.rect?;
        if rect[2] <= 2.0 || rect[3] <= 2.0 {
            return None;
        }
        let fills = self.node_fills.get(&key)?;
        let x0 = fills.iter().map(|f| f[0]).fold(f32::MAX, f32::min);
        let y0 = fills.iter().map(|f| f[1]).fold(f32::MAX, f32::min);
        let x1 = fills.iter().map(|f| f[0] + f[2]).fold(f32::MIN, f32::max);
        let y1 = fills.iter().map(|f| f[1] + f[3]).fold(f32::MIN, f32::max);
        let near = |a: f32, b: f32| (a - b).abs() <= BOX_EDGE;
        (near(x0, rect[0])
            && near(y0, rect[1])
            && near(x1, rect[0] + rect[2])
            && near(y1, rect[1] + rect[3]))
        .then_some(rect)
    }

    fn is_screen(&self, rect: Rect) -> bool {
        rect[2] >= self.screen[2] * 0.95 && rect[3] >= self.screen[3] * 0.95
    }

    /// The nearest painted, not-full-screen box around a node.
    fn enclosing(&self, layer: usize, id: u32, include_self: bool) -> Option<(u32, Rect)> {
        let start = match include_self {
            true => Some(id),
            false => self.nodes.get(&(layer, id)).and_then(|n| n.parent),
        };
        self.ancestors(layer, start).into_iter().find_map(|c| {
            self.painted((layer, c))
                .filter(|r| !self.is_screen(*r))
                .map(|r| (c, r))
        })
    }

    /// Whether an opaque fill drawn strictly between two draws covers `region`.
    fn occluded_between(&self, region: Rect, lo: usize, hi: usize) -> bool {
        self.fills.iter().any(|f| {
            f.seq > lo
                && f.seq < hi
                && f.alpha >= OPAQUE
                && match f.clip {
                    Some(clip) => inter(f.rect, clip),
                    None => Some(f.rect),
                }
                .is_some_and(|visible| contains(visible, region))
        })
    }

    fn run(&self) -> Vec<Finding> {
        let mut out = Vec::new();
        let finding = |kind, path: String, text: &str, amount, ink, other_rect, other: String, detail: String| Finding {
            kind,
            path,
            text: text.to_string(),
            amount,
            ink,
            other_rect,
            other,
            detail,
        };

        // Each run of text on its own: what the player can see of it, and
        // whether that much is inside everything it should be inside.
        let mut visible: Vec<(&Text, Rect, String)> = Vec::new();
        for t in &self.texts {
            let path = self.path(t.layer, t.node);
            if t.dropped > 0 {
                let odd: Vec<String> = t
                    .text
                    .chars()
                    .filter(|c| !c.is_ascii())
                    .map(|c| format!("U+{:04X}", c as u32))
                    .collect();
                out.push(finding("dropped", path.clone(), t.text, t.dropped as f32, Some(t.ink), None, String::new(), odd.join(" ")));
            }
            let mut seen = t.ink;
            if let Some(clip) = t.clip {
                let Some(v) = inter(t.ink, clip) else {
                    continue; // scrolled out of its viewport: not on screen
                };
                seen = v;
                let (cut, side) = excursion(t.ink, clip);
                let name = t.node.map_or("", |id| self.name(t.layer, id));
                if cut > SLACK && !self.is(&self.rules.clip_ok, name) {
                    out.push(finding("clipped", path.clone(), t.text, cut, Some(t.ink), Some(clip), String::new(), side.to_string()));
                }
            }
            let (off, side) = excursion(seen, self.screen);
            if off > SLACK {
                out.push(finding("offscreen", path.clone(), t.text, off, Some(t.ink), None, String::new(), side.to_string()));
            }
            if let Some(id) = t.node {
                if let Some((box_id, rect)) = self.enclosing(t.layer, id, true) {
                    let (spill, side) = excursion(seen, rect);
                    if spill > SLACK {
                        out.push(finding("spill", path.clone(), t.text, spill, Some(t.ink), Some(rect), self.path(t.layer, Some(box_id)), side.to_string()));
                    }
                }
            }
            visible.push((t, seen, path));
        }

        // Text against text: over each other, or close enough to read as one.
        for i in 0..visible.len() {
            let (a, ia, pa) = &visible[i];
            for (b, ib, pb) in visible.iter().skip(i + 1) {
                let (lo, hi) = (a.seq.min(b.seq), a.seq.max(b.seq));
                match inter(*ia, *ib) {
                    None => {
                        // Immediate text (a tooltip, a callout) sits on its
                        // own plate, so a run beside it is across an edge.
                        let common = (ia[1] + ia[3]).min(ib[1] + ib[3]) - ia[1].max(ib[1]);
                        if common < 0.5 * ia[3].min(ib[3]) || a.layer != b.layer {
                            continue;
                        }
                        let gap = ia[0].max(ib[0]) - (ia[0] + ia[2]).min(ib[0] + ib[2]);
                        let limit = CROWD_EMS * a.size.min(b.size);
                        let skip = gap < 0.0
                            || gap >= limit
                            || a.node.is_none()
                            || b.node.is_none()
                            || self.in_popup(a.layer, a.node)
                            || self.in_popup(b.layer, b.node)
                            || self.occluded_between(*ia, lo, hi);
                        if !skip {
                            let (l, r) = if ia[0] <= ib[0] { (a, b) } else { (b, a) };
                            out.push(finding("crowded", pa.clone(), a.text, limit - gap, Some(*ia), Some(*ib), pb.clone(), format!("'{}' then '{}' {gap:.1}px apart", l.text, r.text)));
                        }
                    }
                    Some(region) => {
                        let area = region[2] * region[3];
                        // The same run twice, offset, is a drop shadow.
                        let shadow = pa == pb && a.text == b.text;
                        if area < MIN_OVERLAP_AREA
                            || region[2].min(region[3]) < SLACK
                            || shadow
                            || self.occluded_between(region, lo, hi)
                        {
                            continue;
                        }
                        out.push(finding("overlap", pa.clone(), a.text, area, Some(*ia), Some(*ib), pb.clone(), format!("over '{}'", b.text)));
                    }
                }
            }
        }

        // Boxes, grouped by the painted box they sit in.
        let mut boxes: HashMap<(usize, Option<u32>), Vec<(u32, Rect)>> = HashMap::new();
        let mut keys: Vec<&Key> = self.nodes.keys().collect();
        keys.sort();
        for &(layer, id) in &keys {
            let Some(rect) = self.shown((*layer, *id)) else {
                continue;
            };
            let (off, side) = excursion(rect, self.screen);
            if off > SLACK {
                out.push(finding("offscreen", self.path(*layer, Some(*id)), "", off, Some(rect), None, String::new(), side.to_string()));
            }
            if self.is_screen(rect) {
                continue;
            }
            let parent = self.enclosing(*layer, *id, false);
            if let Some((pid, prect)) = parent {
                let (spill, side) = excursion(rect, prect);
                if spill > SLACK {
                    out.push(finding("box", self.path(*layer, Some(*id)), "", spill, Some(rect), Some(prect), self.path(*layer, Some(pid)), side.to_string()));
                }
            }
            boxes.entry((*layer, parent.map(|p| p.0))).or_default().push((*id, rect));
        }

        // Text running into a box beside it in the same panel.
        for (t, ink, path) in &visible {
            let Some(id) = t.node else {
                continue;
            };
            if self.rules.world_labels.iter().any(|w| path.ends_with(w.as_str())) {
                continue;
            }
            let own = self.enclosing(t.layer, id, true).map(|b| b.0);
            let mine: HashSet<u32> = self.ancestors(t.layer, Some(id)).into_iter().collect();
            for &(box_id, rect) in boxes.get(&(t.layer, own)).into_iter().flatten() {
                if mine.contains(&box_id) || self.is(&self.rules.popups, self.name(t.layer, box_id)) {
                    continue;
                }
                let grown = [
                    rect[0] - COLLIDE_GAP,
                    rect[1] - COLLIDE_GAP,
                    rect[2] + 2.0 * COLLIDE_GAP,
                    rect[3] + 2.0 * COLLIDE_GAP,
                ];
                let Some(hit) = inter(*ink, grown).filter(|h| h[2].min(h[3]) >= 0.5) else {
                    continue;
                };
                let box_seq = self.node_seq.get(&(t.layer, box_id)).copied().unwrap_or(t.seq);
                if self.occluded_between(hit, t.seq.min(box_seq), t.seq.max(box_seq)) {
                    continue;
                }
                let touch = inter(*ink, rect);
                out.push(finding(
                    "collide",
                    path.clone(),
                    t.text,
                    touch.map_or(0.0, |r| r[2].min(r[3])),
                    Some(*ink),
                    Some(rect),
                    self.path(t.layer, Some(box_id)),
                    if touch.is_some() { "under" } else { "touching" }.to_string(),
                ));
            }
        }

        // Boxes over each other in the same panel. A repeater's own instances
        // share a name and may overlap by design (a fanned hand).
        let mut groups: Vec<_> = boxes.iter().collect();
        groups.sort_by_key(|(k, _)| **k);
        for ((layer, _), members) in groups {
            for (i, &(a, ra)) in members.iter().enumerate() {
                for &(b, rb) in &members[i + 1..] {
                    let (na, nb) = (self.name(*layer, a), self.name(*layer, b));
                    if na == nb
                        || self.ancestors(*layer, Some(b)).contains(&a)
                        || self.ancestors(*layer, Some(a)).contains(&b)
                    {
                        continue;
                    }
                    let Some(hit) = inter(ra, rb).filter(|h| h[2].min(h[3]) >= STACK_MIN) else {
                        continue;
                    };
                    let sa = self.node_seq.get(&(*layer, a)).copied().unwrap_or(0);
                    let sb = self.node_seq.get(&(*layer, b)).copied().unwrap_or(0);
                    let (top, under) = if sb > sa { (nb, na) } else { (na, nb) };
                    if self.is(&self.rules.popups, top)
                        || self.is(&self.rules.world_boxes, under)
                        || self.occluded_between(hit, sa.min(sb), sa.max(sb))
                    {
                        continue;
                    }
                    out.push(finding(
                        "stacked",
                        self.path(*layer, Some(a)),
                        "",
                        hit[2].min(hit[3]),
                        Some(ra),
                        Some(rb),
                        self.path(*layer, Some(b)),
                        format!("{:.0}x{:.0}px", hit[2], hit[3]),
                    ));
                }
            }
        }
        out
    }
}

/// Everything in a frame's canvases that does not fit, under `rules`.
pub fn audit_findings(screen: (u32, u32), canvases: &[Canvas], rules: &AuditRules) -> Vec<Finding> {
    Frame::new(screen, canvases, rules).run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::Color;

    /// A canvas with auditing on, whatever the environment says.
    fn canvas() -> Canvas {
        let mut c = Canvas::for_test((200, 100));
        c.force_audit();
        c
    }

    fn kinds(f: &[Finding]) -> Vec<&'static str> {
        f.iter().map(|f| f.kind).collect()
    }

    #[test]
    fn a_child_box_hanging_off_its_panel_is_found() {
        let mut c = canvas();
        // Canvas space: origin at the centre, y up.
        let panel = c.audit_open_node("panel", "rect");
        c.rect(-50.0, -20.0, 100.0, 40.0, Color::WHITE);
        c.audit_node_rect(panel, Some((-50.0, -20.0, 100.0, 40.0)), false);
        let button = c.audit_open_node("button", "rect");
        c.rect(30.0, -10.0, 40.0, 20.0, Color::WHITE);
        c.audit_node_rect(button, Some((30.0, -10.0, 40.0, 20.0)), false);
        c.audit_close_node(button);
        c.audit_close_node(panel);
        let found = audit_findings((200, 100), std::slice::from_ref(&c), &AuditRules::default());
        assert_eq!(kinds(&found), ["box"]);
        assert_eq!(found[0].path, "panel/button");
        assert!((found[0].amount - 20.0).abs() < 0.01, "{found:?}");
    }

    #[test]
    fn a_popup_may_cover_its_neighbours() {
        let mut c = canvas();
        let root = c.audit_open_node("root", "rect");
        c.audit_node_rect(root, Some((-100.0, -50.0, 200.0, 100.0)), false);
        let chip = c.audit_open_node("chip", "rect");
        c.rect(-20.0, -10.0, 40.0, 20.0, Color::WHITE);
        c.audit_node_rect(chip, Some((-20.0, -10.0, 40.0, 20.0)), false);
        c.audit_close_node(chip);
        let modal = c.audit_open_node("modal", "rect");
        c.rect(-30.0, -15.0, 60.0, 30.0, Color::WHITE);
        c.audit_node_rect(modal, Some((-30.0, -15.0, 60.0, 30.0)), false);
        c.audit_close_node(modal);
        c.audit_close_node(root);
        let canvases = std::slice::from_ref(&c);
        assert_eq!(kinds(&audit_findings((200, 100), canvases, &AuditRules::default())), ["stacked"]);
        let rules = AuditRules {
            popups: vec!["modal".into()],
            ..Default::default()
        };
        assert!(audit_findings((200, 100), canvases, &rules).is_empty());
    }
}
