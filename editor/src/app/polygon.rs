//! The polygon tool: draw a shape in the viewport, get an authored node.
//!
//! Set dressing — a grandstand, a gravel trap, a treeline — is a *shape*, and
//! before this the only way to get one into a scene was to type coordinates
//! into `ui_points` by hand or to hand-code its geometry in the host's Rust.
//! Neither is authoring.
//!
//! The tool collects clicks in scene space, then writes one `ui: "polygon"`
//! node whose box is the drawn shape's bounding box and whose points are that
//! shape normalised into it — so the node lands exactly where it was drawn and
//! still scales properly if it is resized afterwards.

use super::*;

/// Below this many points there is no polygon to make.
const MIN_POINTS: usize = 3;

impl RengineNativeEditor {
    /// Whether the viewport is currently collecting polygon points.
    pub(crate) fn polygon_tool_active(&self) -> bool {
        self.polygon_draft.is_some()
    }

    /// Arm the tool. Clicks in the viewport now place points instead of
    /// selecting nodes.
    pub(crate) fn begin_polygon(&mut self) {
        self.polygon_draft = Some(Vec::new());
        self.push_log(
            "Polygon: click to place points, Enter to finish, Esc to cancel".to_string(),
        );
    }

    /// Abandon a half-drawn shape.
    pub(crate) fn cancel_polygon(&mut self) {
        if self.polygon_draft.take().is_some() {
            self.push_log("Polygon cancelled".to_string());
        }
    }

    /// Add a point. Called by the viewport press handler while armed.
    pub(crate) fn push_polygon_point(&mut self, scene_point: [f32; 2]) {
        if let Some(draft) = self.polygon_draft.as_mut() {
            draft.push(scene_point);
        }
    }

    /// Turn the drafted points into a node, and select it.
    ///
    /// The node is `ui_absolute` and anchored by fraction of the scene's own
    /// view box, which is what makes it resolve to the place it was drawn
    /// rather than wherever the flow would otherwise have put it.
    pub(crate) fn finish_polygon(&mut self) {
        let Some(points) = self.polygon_draft.take() else {
            return;
        };
        if points.len() < MIN_POINTS {
            self.push_log(format!(
                "Polygon needs at least {MIN_POINTS} points - discarded"
            ));
            return;
        }

        let view = self.active_scene_tab().scene.view.window_size;
        let props = polygon_node_props(&points, view);

        let parent = self.active_scene_tab().selected_node;
        self.add_node_with_parent(SceneNodeKind::Layout, parent, None);
        let Some(node_id) = self.active_scene_tab().selected_node else {
            return;
        };
        let count = points.len();
        {
            let tab = self.active_scene_tab_mut();
            if let Some(node) = tab.scene.node_mut(node_id) {
                node.name = format!("polygon_{node_id}");
                for (key, value) in props {
                    node.properties.insert(key.to_string(), value);
                }
            }
            tab.mark_dirty();
        }
        self.refresh_inspector_form();
        self.push_log(format!("Polygon: {count} points"));
    }
}

/// The `ui_*` properties that make drawn `points` into an authored node.
///
/// Split out from `finish_polygon` because the editor it lives on needs a
/// window to build, and this arithmetic — the part that can actually be wrong —
/// does not.
fn polygon_node_props(points: &[[f32; 2]], view: [f32; 2]) -> Vec<(&'static str, String)> {
    let (min_x, max_x) = points
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p[0]), hi.max(p[0])));
    let (min_y, max_y) = points
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| (lo.min(p[1]), hi.max(p[1])));
    // A zero-width shape has no box to normalise into; one pixel keeps the
    // division honest and the node still selectable.
    let (w, h) = ((max_x - min_x).max(1.0), (max_y - min_y).max(1.0));
    let ui_points = points
        .iter()
        .map(|p| format!("{:.4},{:.4}", (p[0] - min_x) / w, (p[1] - min_y) / h))
        .collect::<Vec<_>>()
        .join(" ");

    let (view_w, view_h) = (view[0].max(1.0), view[1].max(1.0));
    // Scene space has its origin at the view's centre, so shift into the
    // 0..1 box the anchor fractions are measured in.
    let centre_fx = ((min_x + max_x) * 0.5 + view_w * 0.5) / view_w;
    let centre_fy = ((min_y + max_y) * 0.5 + view_h * 0.5) / view_h;

    vec![
        ("ui", "polygon".to_string()),
        ("ui_points", ui_points),
        ("ui_color", "200,200,200,255".to_string()),
        ("ui_absolute", "true".to_string()),
        ("ui_anchor", "bottom-left".to_string()),
        ("ui_anchor_frac_x", format!("{centre_fx:.4}")),
        ("ui_anchor_frac_y", format!("{centre_fy:.4}")),
        ("ui_origin_x", "0.5".to_string()),
        ("ui_origin_y", "0.5".to_string()),
        ("ui_w", format!("{w:.1}")),
        ("ui_h", format!("{h:.1}")),
    ]
}

#[cfg(test)]
mod tests {
    use super::polygon_node_props;

    fn prop(props: &[(&'static str, String)], key: &str) -> String {
        props
            .iter()
            .find(|(k, _)| *k == key)
            .unwrap_or_else(|| panic!("no {key} in {props:?}"))
            .1
            .clone()
    }

    /// A shape drawn in the viewport must produce the node that draws it back
    /// in the same place. Every expected value here is worked out by hand from
    /// the input, not recomputed from the implementation.
    ///
    /// Triangle (-40,10) (60,10) (60,90) in a 400x300 view: box is 100x80 at
    /// centre (10, 50), which is 0.525 / 0.6667 of the way across a view whose
    /// origin sits at its own centre.
    #[test]
    fn a_drawn_shape_becomes_a_node_at_the_place_it_was_drawn() {
        let props = polygon_node_props(
            &[[-40.0, 10.0], [60.0, 10.0], [60.0, 90.0]],
            [400.0, 300.0],
        );

        assert_eq!(prop(&props, "ui"), "polygon");
        assert_eq!(prop(&props, "ui_points"), "0.0000,0.0000 1.0000,0.0000 1.0000,1.0000");
        assert_eq!(prop(&props, "ui_w"), "100.0");
        assert_eq!(prop(&props, "ui_h"), "80.0");
        assert_eq!(prop(&props, "ui_anchor_frac_x"), "0.5250");
        assert_eq!(prop(&props, "ui_anchor_frac_y"), "0.6667");
        // Without these the node centre-anchors and ignores the fractions
        // entirely - the bug this project has shipped eight times.
        assert_eq!(prop(&props, "ui_absolute"), "true");
        assert_eq!(prop(&props, "ui_origin_x"), "0.5");
        assert_eq!(prop(&props, "ui_origin_y"), "0.5");
    }

    /// A shape drawn flat (all points collinear) must not divide by zero and
    /// leave the node with a NaN size, which would silently drop it.
    #[test]
    fn a_flat_shape_still_produces_a_finite_box() {
        let props =
            polygon_node_props(&[[0.0, 5.0], [50.0, 5.0], [25.0, 5.0]], [400.0, 300.0]);

        assert_eq!(prop(&props, "ui_h"), "1.0");
        for key in ["ui_w", "ui_h", "ui_anchor_frac_x", "ui_anchor_frac_y"] {
            let value = prop(&props, key);
            assert!(
                value.parse::<f32>().is_ok_and(f32::is_finite),
                "{key} was {value}"
            );
        }
        assert!(
            !prop(&props, "ui_points").contains("NaN"),
            "points went NaN: {}",
            prop(&props, "ui_points")
        );
    }
}
