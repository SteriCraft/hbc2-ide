//! The editor groups: the file tabs can be split into several groups side by
//! side, and moved between them by dragging. The other tabs stay in place.

use std::path::Path;

use eframe::egui::{Pos2, Rect};
use egui_dock::{DockState, NodePath, SurfaceIndex, TabIndex};

use crate::tabs::Tab;

/// Where a dragged file tab goes.
#[derive(Clone, Debug, PartialEq)]
pub enum Drop {
    /// Next to `anchor`, in its group: before it, or after it.
    Beside { anchor: Tab, after: bool },
    /// In a new group right of the group of `anchor`.
    SplitRight(Tab),
}

/// A group of file tabs (or the start page, which stands in for them).
fn is_editor_group(tabs: &[Tab]) -> bool {
    tabs.iter()
        .any(|t| matches!(t, Tab::Editor(_) | Tab::Welcome))
}

/// Removes the tabs that match. Each group then shows the same tab as
/// before if it stays, else the closest one on its right, else the closest
/// one on its left. A group left empty goes away.
fn remove_tabs(dock: &mut DockState<Tab>, remove: impl Fn(&Tab) -> bool) {
    let shown: Vec<Tab> = dock
        .iter_leaves()
        .filter_map(|(_, leaf)| {
            let tabs = leaf.tabs();
            let active = leaf.active.0.min(tabs.len().checked_sub(1)?);
            let right = tabs[active..].iter().find(|t| !remove(t));
            right
                .or_else(|| tabs[..active].iter().rev().find(|t| !remove(t)))
                .cloned()
        })
        .collect();
    dock.retain_tabs(|t| !remove(t));
    for (_, leaf) in dock.iter_leaves_mut() {
        if let Some(index) = leaf.tabs().iter().position(|t| shown.contains(t)) {
            leaf.active = TabIndex(index);
        }
    }
}

/// Removes the editor tabs whose path matches. The start page takes the
/// place of the last one, so the editor group never goes away.
pub fn remove_editor_tabs(dock: &mut DockState<Tab>, remove: impl Fn(&Path) -> bool) {
    let editor = |t: &Tab| matches!(t, Tab::Editor(p) if remove(p));
    let kept = dock
        .iter_all_tabs()
        .any(|(_, t)| matches!(t, Tab::Editor(_)) && !editor(t));
    if !kept
        && dock.find_tab(&Tab::Welcome).is_none()
        && let Some(path) = dock.find_tab_from(|t| matches!(t, Tab::Editor(_)))
        && let Ok(leaf) = dock.leaf_mut(path.node_path())
    {
        leaf.append_tab(Tab::Welcome);
    }
    remove_tabs(dock, editor);
}

/// Another tab of the group of `tab`, for [`Drop::SplitRight`]: `None` when
/// `tab` is alone in its group.
pub fn split_anchor(dock: &DockState<Tab>, tab: &Tab) -> Option<Tab> {
    let path = dock.find_tab(tab)?;
    let leaf = dock.leaf(path.node_path()).ok()?;
    leaf.tabs().iter().find(|t| *t != tab).cloned()
}

/// Moves `tab` where `drop` says, shows it and focuses its group.
pub fn move_tab(dock: &mut DockState<Tab>, tab: &Tab, drop: Drop) {
    let anchor = match &drop {
        Drop::Beside { anchor, .. } | Drop::SplitRight(anchor) => anchor.clone(),
    };
    if anchor == *tab || dock.find_tab(tab).is_none() || dock.find_tab(&anchor).is_none() {
        return;
    }
    remove_tabs(dock, |t| t == tab);
    let Some(at) = dock.find_tab(&anchor) else {
        return;
    };
    let node = match drop {
        Drop::Beside { after, .. } => {
            let Ok(leaf) = dock.leaf_mut(at.node_path()) else {
                return;
            };
            leaf.insert_tab(at.tab.0 + after as usize, tab.clone());
            at.node_path()
        }
        Drop::SplitRight(_) => {
            let surface = dock.main_surface_mut();
            let [_, new] = surface.split_right(at.node, 0.5, vec![tab.clone()]);
            NodePath {
                surface: SurfaceIndex::main(),
                node: new,
            }
        }
    };
    dock.set_focused_node_and_surface(node);
}

/// Puts every file tab back in the first editor group (the layout as it is
/// saved between sessions).
pub fn merge_groups(dock: &mut DockState<Tab>) {
    let Some(first) = dock.find_tab_from(|t| matches!(t, Tab::Editor(_))) else {
        return;
    };
    let first = first.node_path();
    let mut moved = Vec::new();
    for (path, leaf) in dock.iter_leaves_mut() {
        if path != first {
            let (editors, rest) = leaf
                .tabs
                .drain(..)
                .partition(|t| matches!(t, Tab::Editor(_)));
            leaf.tabs = rest;
            leaf.active = TabIndex(0);
            moved.extend::<Vec<Tab>>(editors);
        }
    }
    if let Ok(leaf) = dock.leaf_mut(first) {
        leaf.tabs.extend(moved);
    }
    // Takes the emptied groups out of the layout.
    dock.retain_tabs(|_| true);
}

/// Where `dragged` would go if dropped at `pointer`, with the area to
/// highlight. `tabs` holds the rectangles of the file tabs, as drawn.
///
/// Over a tab bar, the tab goes before or after the tab under the pointer
/// (at the end past the last one). Over the right half of a group, it opens
/// in a new group on the right; over the left half of another group, it
/// moves to the end of that group.
pub fn drop_target(
    dock: &DockState<Tab>,
    dragged: &Tab,
    tabs: &[(Tab, Rect)],
    pointer: Pos2,
) -> Option<(Drop, Rect)> {
    let (_, leaf) = dock
        .iter_leaves()
        .filter(|(path, leaf)| path.surface.is_main() && is_editor_group(leaf.tabs()))
        .find(|(_, leaf)| leaf.rect.contains(pointer))?;
    let group: Vec<&(Tab, Rect)> = tabs
        .iter()
        .filter(|(t, _)| leaf.tabs().contains(t))
        .collect();
    let others = || leaf.tabs().iter().filter(|t| *t != dragged);
    let bar = Rect::from_min_max(
        leaf.rect.min,
        Pos2::new(leaf.rect.right(), leaf.viewport.top()),
    );
    let line = |x: f32| Rect::from_x_y_ranges(x - 1.0..=x + 1.0, bar.y_range());

    if bar.contains(pointer) {
        let (anchor, rect, after) = match group.iter().find(|(_, r)| pointer.x < r.right()) {
            Some((tab, rect)) => (tab, *rect, pointer.x > rect.center().x),
            None => {
                let (tab, rect) = group.last()?;
                (tab, *rect, true)
            }
        };
        if anchor == dragged || !matches!(anchor, Tab::Editor(_)) {
            return None;
        }
        let x = if after { rect.right() } else { rect.left() };
        let drop = Drop::Beside {
            anchor: anchor.clone(),
            after,
        };
        return Some((drop, line(x)));
    }

    let body = leaf.viewport;
    if !body.contains(pointer) {
        return None;
    }
    if pointer.x >= body.center().x {
        let anchor = others().next()?;
        let right = Rect::from_min_max(Pos2::new(body.center().x, body.top()), body.max);
        Some((Drop::SplitRight(anchor.clone()), right))
    } else if !leaf.tabs().contains(dragged) {
        let anchor = others().rfind(|t| matches!(t, Tab::Editor(_)))?;
        let drop = Drop::Beside {
            anchor: anchor.clone(),
            after: true,
        };
        Some((drop, body))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn file(name: &str) -> Tab {
        Tab::Editor(PathBuf::from(name))
    }

    fn files(names: &[&str]) -> Vec<Tab> {
        names.iter().map(|n| file(n)).collect()
    }

    /// The tabs of each group, left to right, and the tab each one shows.
    fn groups(dock: &DockState<Tab>) -> Vec<(Vec<Tab>, Tab)> {
        dock.iter_leaves()
            .map(|(_, leaf)| (leaf.tabs().to_vec(), leaf.tabs()[leaf.active.0].clone()))
            .collect()
    }

    /// Opens a, b, c, d with `active` shown, closes `closed`, gives the tab shown.
    fn shown_after(active: usize, closed: &[&str]) -> Tab {
        let mut dock = DockState::new(files(&["a", "b", "c", "d"]));
        dock.iter_leaves_mut().next().unwrap().1.active = TabIndex(active);
        remove_editor_tabs(&mut dock, |p| closed.iter().any(|c| p == Path::new(c)));
        groups(&dock)[0].1.clone()
    }

    #[test]
    fn closing_the_shown_tab_shows_the_one_on_its_right() {
        assert_eq!(shown_after(1, &["b"]), file("c"));
        assert_eq!(shown_after(0, &["a", "b"]), file("c"));
    }

    #[test]
    fn closing_the_last_tab_shows_the_one_on_its_left() {
        assert_eq!(shown_after(3, &["d"]), file("c"));
        assert_eq!(shown_after(2, &["c", "d"]), file("b"));
    }

    #[test]
    fn closing_another_tab_keeps_the_shown_one() {
        assert_eq!(shown_after(2, &["a"]), file("c"));
        assert_eq!(shown_after(1, &["d"]), file("b"));
    }

    #[test]
    fn closing_every_tab_shows_the_start_page() {
        assert_eq!(shown_after(2, &["a", "b", "c", "d"]), Tab::Welcome);
    }

    #[test]
    fn a_tab_moves_within_its_group() {
        let mut dock = DockState::new(files(&["a", "b", "c"]));
        let drop = Drop::Beside {
            anchor: file("c"),
            after: true,
        };
        move_tab(&mut dock, &file("a"), drop);
        assert_eq!(groups(&dock), vec![(files(&["b", "c", "a"]), file("a"))]);
        let drop = Drop::Beside {
            anchor: file("b"),
            after: false,
        };
        move_tab(&mut dock, &file("a"), drop);
        assert_eq!(groups(&dock), vec![(files(&["a", "b", "c"]), file("a"))]);
    }

    #[test]
    fn split_right_opens_the_tab_in_a_new_group() {
        let mut dock = DockState::new(files(&["a", "b", "c"]));
        dock.iter_leaves_mut().next().unwrap().1.active = TabIndex(1);
        assert_eq!(split_anchor(&dock, &file("b")), Some(file("a")));
        move_tab(&mut dock, &file("b"), Drop::SplitRight(file("a")));
        let expected = vec![(files(&["a", "c"]), file("c")), (files(&["b"]), file("b"))];
        assert_eq!(groups(&dock), expected);
        let focused = dock.focused_leaf().unwrap();
        assert_eq!(dock.leaf(focused).unwrap().tabs(), files(&["b"]).as_slice());
        // Alone in its group, it cannot be split from itself.
        assert_eq!(split_anchor(&dock, &file("b")), None);
    }

    #[test]
    fn a_group_emptied_by_a_move_goes_away() {
        let mut dock = DockState::new(files(&["a", "b"]));
        move_tab(&mut dock, &file("b"), Drop::SplitRight(file("a")));
        let drop = Drop::Beside {
            anchor: file("a"),
            after: false,
        };
        move_tab(&mut dock, &file("b"), drop);
        assert_eq!(groups(&dock), vec![(files(&["b", "a"]), file("b"))]);
    }

    #[test]
    fn merging_puts_every_file_back_in_the_first_group() {
        let mut dock = DockState::new(files(&["a", "b", "c"]));
        move_tab(&mut dock, &file("b"), Drop::SplitRight(file("a")));
        merge_groups(&mut dock);
        assert_eq!(groups(&dock)[0].0, files(&["a", "c", "b"]));
        assert_eq!(groups(&dock).len(), 1);
    }

    #[test]
    fn drop_targets() {
        let mut dock = DockState::new(files(&["a", "b"]));
        {
            let (_, leaf) = dock.iter_leaves_mut().next().unwrap();
            leaf.rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(400.0, 300.0));
            leaf.viewport = Rect::from_min_max(Pos2::new(0.0, 20.0), Pos2::new(400.0, 300.0));
        }
        let tab_rect = |x: f32| Rect::from_min_max(Pos2::new(x, 0.0), Pos2::new(x + 50.0, 20.0));
        let tabs = vec![(file("a"), tab_rect(0.0)), (file("b"), tab_rect(50.0))];
        let target = |x: f32, y: f32| {
            drop_target(&dock, &file("a"), &tabs, Pos2::new(x, y)).map(|(drop, _)| drop)
        };
        // Tab bar: right half of b, then past the last tab.
        let after_b = Some(Drop::Beside {
            anchor: file("b"),
            after: true,
        });
        assert_eq!(target(90.0, 10.0), after_b);
        assert_eq!(target(300.0, 10.0), after_b);
        // Over itself: nothing.
        assert_eq!(target(10.0, 10.0), None);
        // Right half of the body: split; left half of its own group: nothing.
        assert_eq!(target(300.0, 100.0), Some(Drop::SplitRight(file("b"))));
        assert_eq!(target(100.0, 100.0), None);
    }
}
