//! The UI audit: what a canvas actually drew, written down so a tool can ask
//! whether every run of text sits inside its box.
//!
//! Set `RENGINE_UI_AUDIT=1` and every [`Canvas`](super::Canvas) keeps a record
//! of its frame: each scene node it laid out (name, parent, resolved rect,
//! whether it clips), each run of text (the ink it really put down, measured
//! glyph by glyph in the face and at the scale it was painted at, plus the clip
//! it was painted under), and each filled rect or image (so a reader can tell
//! a painted box from an empty layout slot, and what covers what). A headless
//! capture writes the frame's records beside its PNG as
//! `<capture>.audit.jsonl`.
//!
//! Why the ink and not the layout: a node's rect is what the layout *meant*;
//! the glyphs are what the player sees. A content-sized box, a hand-counted
//! literal width, a wrap that runs on untracked widths and a clip that cuts a
//! line in half all disagree with each other in ways only the painted result
//! settles. Measuring it here, where every text path ends, catches all of them
//! with one hook, scene-authored or immediate-mode alike.
//!
//! Off, it costs one branch per draw call. Coordinates in the records are
//! screen pixels, origin top-left, y down: the same space as the capture, so a
//! reader can draw them straight onto it.

use serde::Serialize;

/// Whether `RENGINE_UI_AUDIT` is set (to anything but `0`/`false`/`off`).
/// Read once per process.
pub fn audit_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| match std::env::var("RENGINE_UI_AUDIT") {
        Ok(value) => !matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "off" | "no"
        ),
        Err(_) => false,
    })
}

/// Whether canvases keep their records at all: under the audit, and under a
/// play script, whose `click @node` lines find their target in the previous
/// frame's node rects. A play script alone records but writes nothing; only
/// the audit writes `.audit.jsonl` beside a capture.
pub fn node_records_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| {
        audit_enabled() || std::env::var_os("RENGINE_PLAY_SCRIPT").is_some()
    })
}

/// Whether `RENGINE_UI_AUDIT=check`: a capture then also prints what does not
/// fit to stderr (`audit_check.rs`), which is how a test harness that wants an
/// empty stderr turns every capture into a fit check.
pub fn audit_check_mode() -> bool {
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *CHECK.get_or_init(|| {
        std::env::var("RENGINE_UI_AUDIT").is_ok_and(|v| v.trim().eq_ignore_ascii_case("check"))
    })
}

/// One thing a canvas drew. Rects are `[x, y, w, h]` in screen pixels.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum AuditRecord {
    /// A scene node laid out on this canvas. `rect` is `None` when the node
    /// resolved invisible (`ui_visible: false`) or has no `ui` kind.
    Node {
        id: u32,
        parent: Option<u32>,
        name: String,
        kind: String,
        rect: Option<[f32; 4]>,
        clips: bool,
    },
    /// A run of text. `ink` is the union of its glyph quads (`None` for a run
    /// with no visible glyphs); `dropped` counts characters the face could not
    /// draw, which the text paths skip silently.
    Text {
        node: Option<u32>,
        text: String,
        size: f32,
        ink: Option<[f32; 4]>,
        clip: Option<[f32; 4]>,
        dropped: u32,
    },
    /// A filled rect or an image, with its opacity.
    Fill {
        node: Option<u32>,
        rect: [f32; 4],
        alpha: f32,
        clip: Option<[f32; 4]>,
    },
}

/// A canvas's records for one frame, and the node it is drawing right now.
#[derive(Debug, Default)]
pub struct UiAudit {
    pub records: Vec<AuditRecord>,
    /// The open nodes, innermost last: whatever is drawn now belongs to the
    /// last one.
    stack: Vec<u32>,
    next_id: u32,
}

impl UiAudit {
    pub(crate) fn current_node(&self) -> Option<u32> {
        self.stack.last().copied()
    }

    /// Open a node: everything drawn until [`close_node`](Self::close_node)
    /// belongs to it, and nodes opened meanwhile are its children.
    pub(crate) fn open_node(&mut self, name: String, kind: String) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.records.push(AuditRecord::Node {
            id,
            parent: self.current_node(),
            name,
            kind,
            rect: None,
            clips: false,
        });
        self.stack.push(id);
        id
    }

    /// Fill in an open node's resolved rect once its layout is known.
    pub(crate) fn set_node_rect(&mut self, id: u32, rect: Option<[f32; 4]>, clips: bool) {
        for record in self.records.iter_mut().rev() {
            if let AuditRecord::Node {
                id: rid,
                rect: r,
                clips: c,
                ..
            } = record
            {
                if *rid == id {
                    *r = rect;
                    *c = clips;
                    return;
                }
            }
        }
    }

    pub(crate) fn close_node(&mut self) {
        self.stack.pop();
    }
}

/// Grows a glyph-by-glyph ink box. Coordinates in, screen pixels out.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct InkBox {
    min: Option<(f32, f32)>,
    max: (f32, f32),
}

impl InkBox {
    /// Add a quad given in canvas space (origin centre, y up).
    pub(crate) fn add(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let (x1, y1) = (x + w, y + h);
        match &mut self.min {
            None => {
                self.min = Some((x, y));
                self.max = (x1, y1);
            }
            Some(min) => {
                min.0 = min.0.min(x);
                min.1 = min.1.min(y);
                self.max.0 = self.max.0.max(x1);
                self.max.1 = self.max.1.max(y1);
            }
        }
    }

    pub(crate) fn screen_rect(&self, screen: (u32, u32)) -> Option<[f32; 4]> {
        let (x0, y0) = self.min?;
        let (x1, y1) = self.max;
        Some(canvas_rect_to_screen(x0, y0, x1 - x0, y1 - y0, screen))
    }
}

/// A canvas-space rect (origin at the centre, y up, `(x, y)` its bottom-left)
/// as screen pixels (origin top-left, y down, `[x, y]` its top-left).
pub(crate) fn canvas_rect_to_screen(x: f32, y: f32, w: f32, h: f32, screen: (u32, u32)) -> [f32; 4] {
    let hw = screen.0 as f32 / 2.0;
    let hh = screen.1 as f32 / 2.0;
    [x + hw, hh - (y + h), w, h]
}

/// Write a frame's canvases' records as JSON lines: a header with the screen
/// size, then every record tagged with the index of the canvas that drew it
/// (canvases composite in index order, so that is the layer), then what does
/// not fit (`{"t":"finding",...}`, from `audit_check.rs`).
pub fn write_audit(
    path: &std::path::Path,
    screen: (u32, u32),
    canvases: &[super::Canvas],
    findings: &[super::Finding],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut out = std::io::BufWriter::new(std::fs::File::create(path)?);
    writeln!(
        out,
        "{}",
        serde_json::json!({ "t": "screen", "w": screen.0, "h": screen.1 })
    )?;
    for (layer, canvas) in canvases.iter().enumerate() {
        let Some(audit) = canvas.audit() else {
            continue;
        };
        for record in &audit.records {
            let mut value = serde_json::to_value(record).map_err(std::io::Error::other)?;
            value["layer"] = serde_json::json!(layer);
            writeln!(out, "{value}")?;
        }
    }
    for finding in findings {
        let mut value = serde_json::to_value(finding).map_err(std::io::Error::other)?;
        value["t"] = serde_json::json!("finding");
        writeln!(out, "{value}")?;
    }
    out.flush()
}

/// The audit file beside a capture: `shot.png` -> `shot.png.audit.jsonl`.
pub fn audit_path_for(capture: &std::path::Path) -> std::path::PathBuf {
    let mut name = capture.as_os_str().to_owned();
    name.push(".audit.jsonl");
    std::path::PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_space_maps_to_screen_pixels() {
        // A 10x4 rect whose bottom-left is the centre of a 100x50 screen sits
        // with its top-left 4px above the centre.
        assert_eq!(
            canvas_rect_to_screen(0.0, 0.0, 10.0, 4.0, (100, 50)),
            [50.0, 21.0, 10.0, 4.0]
        );
    }

    #[test]
    fn an_ink_box_is_the_union_of_its_quads() {
        let mut ink = InkBox::default();
        assert!(ink.screen_rect((100, 50)).is_none());
        ink.add(0.0, 0.0, 5.0, 5.0);
        ink.add(10.0, -2.0, 5.0, 5.0);
        assert_eq!(ink.screen_rect((100, 50)), Some([50.0, 20.0, 15.0, 7.0]));
    }

    #[test]
    fn nodes_nest_and_draws_belong_to_the_innermost() {
        let mut audit = UiAudit::default();
        let outer = audit.open_node("panel".into(), "rect".into());
        let inner = audit.open_node("label".into(), "text".into());
        assert_eq!(audit.current_node(), Some(inner));
        audit.close_node();
        assert_eq!(audit.current_node(), Some(outer));
        audit.set_node_rect(inner, Some([1.0, 2.0, 3.0, 4.0]), false);
        let AuditRecord::Node { parent, rect, .. } = &audit.records[1] else {
            panic!("expected a node");
        };
        assert_eq!(*parent, Some(outer));
        assert_eq!(*rect, Some([1.0, 2.0, 3.0, 4.0]));
    }
}
