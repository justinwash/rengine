//! Immediate-mode layout helpers that compute [`Rect`]s — they never draw.
//!
//! The engine's UI is drawn with the immediate-mode canvas API and positioned
//! in canvas space (center origin, **y-up**: larger `y` is higher on screen,
//! matching [`Rect`] and `Engine::mouse_canvas_pos`). Historically screens hand-
//! placed every element with absolute coordinates, so anything that changed size
//! or moved either collided or floated in dead space.
//!
//! These helpers replace that: you take a region (typically `Engine::canvas_rect`
//! or a slice of it), then inset / anchor / split / distribute it into child
//! `Rect`s that your existing `canvas.text`/`rect`/`image` calls draw into.
//! Because a [`Stack`] *consumes* space as it hands out slices, siblings cannot
//! overlap; because regions anchor to their parent, they cannot drift into dead
//! space. Tunable values (padding, spacing, item sizes) are plain `f32`
//! parameters, so a game can still feed them from editor-authored metrics.

use crate::math::Rect;

/// Nine-point anchor for placing a fixed-size box inside a region. Vertical
/// names follow the y-up canvas: `Top*` is the high-`y` edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// Placement factors in `[0, 1]`: `fx` from left, `fy` from **bottom** (y-up).
    fn factors(self) -> (f32, f32) {
        use Anchor::*;
        let fx = match self {
            TopLeft | Left | BottomLeft => 0.0,
            Top | Center | Bottom => 0.5,
            TopRight | Right | BottomRight => 1.0,
        };
        let fy = match self {
            BottomLeft | Bottom | BottomRight => 0.0,
            Left | Center | Right => 0.5,
            TopLeft | Top | TopRight => 1.0,
        };
        (fx, fy)
    }
}

/// A sizing rule for one slot along a distributed axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Track {
    /// Share leftover space equally with the other flexible slots (weight 1).
    Even,
    /// An exact main-axis size in pixels; ignores leftover space.
    Fixed(f32),
    /// Share leftover space in proportion to this weight.
    Weight(f32),
}

impl Track {
    fn weight(self) -> f32 {
        match self {
            Track::Even => 1.0,
            Track::Weight(w) => w.max(0.0),
            Track::Fixed(_) => 0.0,
        }
    }
    fn fixed(self) -> f32 {
        match self {
            Track::Fixed(px) => px.max(0.0),
            _ => 0.0,
        }
    }
}

/// How to spread slack when placing fixed-size items along an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

/// Main-axis sizes for `count` slots given `tracks`, `spacing`, and total `len`.
fn resolve_tracks(tracks: &[Track], spacing: f32, len: f32) -> Vec<f32> {
    let n = tracks.len();
    if n == 0 {
        return Vec::new();
    }
    let gaps = spacing * (n.saturating_sub(1)) as f32;
    let fixed: f32 = tracks.iter().map(|t| t.fixed()).sum();
    let weight: f32 = tracks.iter().map(|t| t.weight()).sum();
    let flex = (len - gaps - fixed).max(0.0);
    tracks
        .iter()
        .map(|t| match t {
            Track::Fixed(px) => px.max(0.0),
            _ if weight > 0.0 => flex * t.weight() / weight,
            _ => 0.0,
        })
        .collect()
}

/// Leading offset and inter-item gap for a `Justify` given `count` items whose
/// sizes sum to `content` within `len`.
fn justify_offsets(justify: Justify, count: usize, content: f32, len: f32) -> (f32, f32) {
    let slack = (len - content).max(0.0);
    let n = count as f32;
    match justify {
        Justify::Start => (0.0, 0.0),
        Justify::Center => (slack / 2.0, 0.0),
        Justify::End => (slack, 0.0),
        Justify::SpaceBetween if count > 1 => (0.0, slack / (n - 1.0)),
        Justify::SpaceBetween => (slack / 2.0, 0.0),
        Justify::SpaceAround if count > 0 => (slack / n / 2.0, slack / n),
        Justify::SpaceAround => (0.0, 0.0),
        Justify::SpaceEvenly => (slack / (n + 1.0), slack / (n + 1.0)),
    }
}

/// Layout operations on a region. All results stay inside `self` (a bigger
/// child than its parent is clamped by the available length, not by panicking).
impl Rect {
    /// Shrink by `pad` on every side (never past zero size).
    pub fn inset(&self, pad: f32) -> Rect {
        self.inset_xy(pad, pad)
    }

    /// Shrink by `px` on the left/right and `py` on the top/bottom.
    pub fn inset_xy(&self, px: f32, py: f32) -> Rect {
        Rect::new(
            self.x + px,
            self.y + py,
            (self.width - 2.0 * px).max(0.0),
            (self.height - 2.0 * py).max(0.0),
        )
    }

    /// Place a `w`×`h` box inside `self` at `anchor`, inset from the anchored
    /// edges by `margin`. Useful for a Back button (`Anchor::Bottom`), a badge
    /// (`Anchor::TopRight`), etc.
    pub fn place(&self, w: f32, h: f32, anchor: Anchor, margin: f32) -> Rect {
        let inner = self.inset(margin);
        let (fx, fy) = anchor.factors();
        Rect::new(
            inner.x + (inner.width - w) * fx,
            inner.y + (inner.height - h) * fy,
            w,
            h,
        )
    }

    /// Split into `n` equal columns (left→right) separated by `spacing`.
    pub fn split_h(&self, n: usize, spacing: f32) -> Vec<Rect> {
        self.distribute_h(&vec![Track::Even; n], spacing)
    }

    /// Split into `n` equal rows (top→bottom) separated by `spacing`.
    pub fn split_v(&self, n: usize, spacing: f32) -> Vec<Rect> {
        self.distribute_v(&vec![Track::Even; n], spacing)
    }

    /// Columns (left→right) sized by `tracks`: `Fixed` slots take their pixels,
    /// the rest share the remainder by weight (`Even` == weight 1).
    pub fn distribute_h(&self, tracks: &[Track], spacing: f32) -> Vec<Rect> {
        let sizes = resolve_tracks(tracks, spacing, self.width);
        let mut out = Vec::with_capacity(sizes.len());
        let mut x = self.x;
        for w in sizes {
            out.push(Rect::new(x, self.y, w, self.height));
            x += w + spacing;
        }
        out
    }

    /// Rows (top→bottom) sized by `tracks`; see [`Rect::distribute_h`]. The first
    /// row is at the top (highest `y`), matching reading order in the y-up canvas.
    pub fn distribute_v(&self, tracks: &[Track], spacing: f32) -> Vec<Rect> {
        let sizes = resolve_tracks(tracks, spacing, self.height);
        let mut out = Vec::with_capacity(sizes.len());
        let mut top = self.top();
        for h in sizes {
            out.push(Rect::new(self.x, top - h, self.width, h));
            top -= h + spacing;
        }
        out
    }

    /// Place fixed-size items (left→right) of the given widths, spreading any
    /// slack according to `justify`. Cross-axis fills `self.height`.
    pub fn justify_h(&self, sizes: &[f32], justify: Justify) -> Vec<Rect> {
        let content: f32 = sizes.iter().map(|s| s.max(0.0)).sum();
        let (lead, gap) = justify_offsets(justify, sizes.len(), content, self.width);
        let mut out = Vec::with_capacity(sizes.len());
        let mut x = self.x + lead;
        for &w in sizes {
            out.push(Rect::new(x, self.y, w.max(0.0), self.height));
            x += w.max(0.0) + gap;
        }
        out
    }

    /// Place fixed-size items (top→bottom) of the given heights, spreading slack
    /// according to `justify`. Cross-axis fills `self.width`.
    pub fn justify_v(&self, sizes: &[f32], justify: Justify) -> Vec<Rect> {
        let content: f32 = sizes.iter().map(|s| s.max(0.0)).sum();
        let (lead, gap) = justify_offsets(justify, sizes.len(), content, self.height);
        let mut out = Vec::with_capacity(sizes.len());
        let mut top = self.top() - lead;
        for &h in sizes {
            out.push(Rect::new(self.x, top - h.max(0.0), self.width, h.max(0.0)));
            top -= h.max(0.0) + gap;
        }
        out
    }
}

/// A cursor that hands out slices of a region in flow order, consuming space as
/// it goes so slices never overlap. Vertical stacks flow top→bottom.
#[derive(Debug, Clone, Copy)]
pub struct Stack {
    area: Rect,
    vertical: bool,
    spacing: f32,
    used: f32,
}

impl Stack {
    /// A top→bottom stack over `area` with `spacing` between slices.
    pub fn vertical(area: Rect, spacing: f32) -> Self {
        Self {
            area,
            vertical: true,
            spacing,
            used: 0.0,
        }
    }

    /// A left→right stack over `area` with `spacing` between slices.
    pub fn horizontal(area: Rect, spacing: f32) -> Self {
        Self {
            area,
            vertical: false,
            spacing,
            used: 0.0,
        }
    }

    /// Take the next slice of `extent` (height for vertical, width for
    /// horizontal), spanning the full cross-axis, then advance past it + spacing.
    pub fn next(&mut self, extent: f32) -> Rect {
        let r = if self.vertical {
            let top = self.area.top() - self.used;
            Rect::new(self.area.x, top - extent, self.area.width, extent)
        } else {
            let left = self.area.x + self.used;
            Rect::new(left, self.area.y, extent, self.area.height)
        };
        self.used += extent + self.spacing;
        r
    }

    /// Advance by exactly `extent` without emitting a slice (a bare spacer, no
    /// extra spacing added).
    pub fn skip(&mut self, extent: f32) {
        self.used += extent;
    }

    /// The space not yet consumed — hand the rest of the column to a sub-layout.
    pub fn remaining(&self) -> Rect {
        if self.vertical {
            let top = self.area.top() - self.used;
            Rect::new(
                self.area.x,
                self.area.y,
                self.area.width,
                (top - self.area.y).max(0.0),
            )
        } else {
            let left = self.area.x + self.used;
            Rect::new(
                left,
                self.area.y,
                (self.area.right() - left).max(0.0),
                self.area.height,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }
    fn rect_eq(r: Rect, x: f32, y: f32, w: f32, h: f32) {
        assert!(
            approx(r.x, x) && approx(r.y, y) && approx(r.width, w) && approx(r.height, h),
            "expected ({x}, {y}, {w}, {h}), got ({}, {}, {}, {})",
            r.x,
            r.y,
            r.width,
            r.height
        );
    }

    #[test]
    fn inset_shrinks_and_clamps() {
        rect_eq(
            Rect::new(0.0, 0.0, 100.0, 60.0).inset(10.0),
            10.0,
            10.0,
            80.0,
            40.0,
        );
        // Over-inset clamps to zero size rather than going negative.
        rect_eq(
            Rect::new(0.0, 0.0, 10.0, 10.0).inset(20.0),
            20.0,
            20.0,
            0.0,
            0.0,
        );
    }

    #[test]
    fn place_anchors_in_y_up_space() {
        let area = Rect::new(-320.0, -180.0, 640.0, 360.0); // a 640x360 canvas
                                                            // Bottom-center Back button, 120x32, 8px margin: centered in x, near the
                                                            // low-y edge.
        rect_eq(
            area.place(120.0, 32.0, Anchor::Bottom, 8.0),
            -60.0,
            -172.0,
            120.0,
            32.0,
        );
        // Top-right badge sits at the high-y, high-x corner.
        let tr = area.place(40.0, 40.0, Anchor::TopRight, 8.0);
        rect_eq(tr, 320.0 - 8.0 - 40.0, 180.0 - 8.0 - 40.0, 40.0, 40.0);
    }

    #[test]
    fn split_v_is_even_top_to_bottom() {
        let rows = Rect::new(0.0, 0.0, 100.0, 100.0).split_v(2, 20.0);
        assert_eq!(rows.len(), 2);
        // Two 40px rows with a 20px gap; first row is the top one.
        rect_eq(rows[0], 0.0, 60.0, 100.0, 40.0);
        rect_eq(rows[1], 0.0, 0.0, 100.0, 40.0);
    }

    #[test]
    fn distribute_mixes_fixed_and_flex() {
        // 300 wide, one 100px fixed column + two even columns, no spacing.
        let cols = Rect::new(0.0, 0.0, 300.0, 50.0)
            .distribute_h(&[Track::Fixed(100.0), Track::Even, Track::Even], 0.0);
        rect_eq(cols[0], 0.0, 0.0, 100.0, 50.0);
        rect_eq(cols[1], 100.0, 0.0, 100.0, 50.0);
        rect_eq(cols[2], 200.0, 0.0, 100.0, 50.0);
        // Weight doubles one flex column's share.
        let cols =
            Rect::new(0.0, 0.0, 300.0, 50.0).distribute_h(&[Track::Weight(2.0), Track::Even], 0.0);
        rect_eq(cols[0], 0.0, 0.0, 200.0, 50.0);
        rect_eq(cols[1], 200.0, 0.0, 100.0, 50.0);
    }

    #[test]
    fn justify_spreads_slack_as_gaps() {
        // Three 20px items in 100px: SpaceBetween -> gaps of 20 between them.
        let items =
            Rect::new(0.0, 0.0, 100.0, 10.0).justify_h(&[20.0, 20.0, 20.0], Justify::SpaceBetween);
        rect_eq(items[0], 0.0, 0.0, 20.0, 10.0);
        rect_eq(items[1], 40.0, 0.0, 20.0, 10.0);
        rect_eq(items[2], 80.0, 0.0, 20.0, 10.0);
        // Center offsets the block by half the slack (100 - 40 = 60 -> 30 lead).
        let items = Rect::new(0.0, 0.0, 100.0, 10.0).justify_h(&[20.0, 20.0], Justify::Center);
        rect_eq(items[0], 30.0, 0.0, 20.0, 10.0);
        rect_eq(items[1], 50.0, 0.0, 20.0, 10.0);
    }

    #[test]
    fn stack_consumes_space_without_overlap() {
        let area = Rect::new(0.0, 0.0, 200.0, 100.0);
        let mut col = Stack::vertical(area, 10.0);
        let a = col.next(30.0);
        let b = col.next(20.0);
        rect_eq(a, 0.0, 70.0, 200.0, 30.0); // top slice
        rect_eq(b, 0.0, 40.0, 200.0, 20.0); // below a, past the 10px gap
        assert!(!a.overlaps(&b));
        // Remaining is everything below what was used (30 + 10 + 20 + 10 = 70).
        rect_eq(col.remaining(), 0.0, 0.0, 200.0, 30.0);
    }
}
