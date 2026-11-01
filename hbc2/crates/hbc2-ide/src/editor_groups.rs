//! The editor groups: the file tabs can be split into several groups side by
//! side, and moved between them by dragging. The other tabs stay in place.

use std::path::{Path, PathBuf};

use eframe::egui::{Pos2, Rect};
use egui_dock::{DockState, Node, NodeIndex, NodePath, SurfaceIndex, TabIndex, Tree};
use serde::{Deserialize, Serialize};

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

/// The editor groups as saved between sessions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EditorLayout {
    /// One group: its files, in tab order, and the file it shows.
    Group {
        files: Vec<PathBuf>,
        shown: Option<PathBuf>,
    },
    /// Two parts side by side; `fraction` is the share of the left one.
    Split {
        fraction: f32,
        left: Box<EditorLayout>,
        right: Box<EditorLayout>,
    },
}

impl EditorLayout {
    fn files(&self) -> Vec<PathBuf> {
        match self {
            EditorLayout::Group { files, .. } => files.clone(),
            EditorLayout::Split { left, right, .. } => {
                let mut files = left.files();
                files.extend(right.files());
                files
            }
        }
    }

    /// Without the files that `keep` refuses; `None` if none is left.
    fn retain(self, keep: &impl Fn(&Path) -> bool) -> Option<EditorLayout> {
        match self {
            EditorLayout::Group { mut files, shown } => {
                files.retain(|f| keep(f));
                let shown = shown.filter(|s| files.contains(s));
                (!files.is_empty()).then_some(EditorLayout::Group { files, shown })
            }
            EditorLayout::Split {
                fraction,
                left,
                right,
            } => match (left.retain(keep), right.retain(keep)) {
                (Some(left), Some(right)) => Some(EditorLayout::Split {
                    fraction,
                    left: Box::new(left),
                    right: Box::new(right),
                }),
                (side, None) | (None, side) => side,
            },
        }
    }
}

/// The editor groups of `dock`, `None` without file tabs.
pub fn editor_layout(dock: &DockState<Tab>) -> Option<EditorLayout> {
    // The part of the layout that holds every editor group.
    let mut nodes = dock
        .iter_leaves()
        .filter(|(path, leaf)| {
            path.surface.is_main() && leaf.tabs().iter().any(|t| matches!(t, Tab::Editor(_)))
        })
        .map(|(path, _)| path.node);
    let first = nodes.next()?;
    let root = nodes.fold(first, common_ancestor);
    Some(layout_of(dock.main_surface(), root))
}

fn common_ancestor(mut a: NodeIndex, mut b: NodeIndex) -> NodeIndex {
    while a != b {
        if a.level() < b.level() {
            std::mem::swap(&mut a, &mut b);
        }
        match a.parent() {
            Some(parent) => a = parent,
            None => return a,
        }
    }
    a
}

fn layout_of(tree: &Tree<Tab>, node: NodeIndex) -> EditorLayout {
    match &tree[node] {
        Node::Horizontal(split) | Node::Vertical(split) => EditorLayout::Split {
            fraction: split.fraction,
            left: Box::new(layout_of(tree, node.left())),
            right: Box::new(layout_of(tree, node.right())),
        },
        Node::Leaf(leaf) => {
            let path = |t: &Tab| match t {
                Tab::Editor(path) => Some(path.clone()),
                _ => None,
            };
            EditorLayout::Group {
                files: leaf.tabs().iter().filter_map(path).collect(),
                shown: leaf.tabs().get(leaf.active.0).and_then(path),
            }
        }
        Node::Empty => EditorLayout::Group {
            files: Vec::new(),
            shown: None,
        },
    }
}

/// Splits the file tabs, all in one group, as `layout` says. Files of
/// `layout` without a tab are left out; tabs it does not name stay in the
/// first group.
pub fn restore_layout(dock: &mut DockState<Tab>, layout: EditorLayout) {
    let open: Vec<PathBuf> = dock
        .iter_all_tabs()
        .filter_map(|(_, t)| match t {
            Tab::Editor(path) => Some(path.clone()),
            _ => None,
        })
        .collect();
    let Some(layout) = layout.retain(&|f| open.iter().any(|o| o == f)) else {
        return;
    };
    let Some(at) = dock.find_tab_from(|t| matches!(t, Tab::Editor(_))) else {
        return;
    };
    if !at.surface.is_main() {
        return;
    }
    build(dock.main_surface_mut(), at.node, &layout);
}

fn build(tree: &mut Tree<Tab>, node: NodeIndex, layout: &EditorLayout) {
    let Node::Leaf(leaf) = &mut tree[node] else {
        return;
    };
    match layout {
        EditorLayout::Group { files, shown } => {
            let place = |t: &Tab| match t {
                Tab::Editor(path) => files.iter().position(|f| f == path),
                _ => None,
            };
            leaf.tabs.sort_by_key(|t| place(t).unwrap_or(usize::MAX));
            let shown = shown.as_ref().and_then(|s| {
                leaf.tabs
                    .iter()
                    .position(|t| matches!(t, Tab::Editor(p) if p == s))
            });
            leaf.active = TabIndex(shown.unwrap_or(0));
        }
        EditorLayout::Split {
            fraction,
            left,
            right,
        } => {
            let moved = right.files();
            let (right_tabs, rest): (Vec<Tab>, Vec<Tab>) = leaf
                .tabs
                .drain(..)
                .partition(|t| matches!(t, Tab::Editor(p) if moved.contains(p)));
            leaf.tabs = rest;
            leaf.active = TabIndex(0);
            if leaf.tabs.is_empty() || right_tabs.is_empty() {
                // An empty group is not allowed: keep them together.
                leaf.tabs.extend(right_tabs);
                return;
            }
            let fraction = fraction.clamp(0.05, 0.95);
            let [left_node, right_node] = tree.split_right(node, fraction, right_tabs);
            build(tree, left_node, left);
            build(tree, right_node, right);
        }
    }
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
    fn the_editor_layout_comes_back() {
        // a | b, c | d, with c shown, beside other tabs.
        let mut dock = crate::app::default_layout();
        for name in ["a", "b", "c", "d"] {
            let welcome = dock.find_tab(&Tab::Welcome).unwrap();
            dock.leaf_mut(welcome.node_path())
                .unwrap()
                .append_tab(file(name));
        }
        let welcome = dock.find_tab(&Tab::Welcome).unwrap();
        dock.remove_tab(welcome);
        move_tab(&mut dock, &file("b"), Drop::SplitRight(file("a")));
        let drop = Drop::Beside {
            anchor: file("b"),
            after: true,
        };
        move_tab(&mut dock, &file("c"), drop);
        move_tab(&mut dock, &file("d"), Drop::SplitRight(file("b")));
        let at = dock.find_tab(&file("c")).unwrap();
        dock.set_active_tab(at).unwrap();
        let layout = editor_layout(&dock).unwrap();
        let editors = |dock: &DockState<Tab>| {
            groups(dock)
                .into_iter()
                .filter(|(tabs, _)| tabs.iter().all(|t| matches!(t, Tab::Editor(_))))
                .collect::<Vec<_>>()
        };

        // Saved as one group, then split again.
        let mut restored = dock.clone();
        merge_groups(&mut restored);
        assert_eq!(editors(&restored).len(), 1);
        restore_layout(&mut restored, layout.clone());
        assert_eq!(editor_layout(&restored), Some(layout.clone()));
        assert_eq!(editors(&restored), editors(&dock));

        // A file gone from the disk takes its group away.
        let mut restored = dock.clone();
        merge_groups(&mut restored);
        let d = restored.find_tab(&file("d")).unwrap();
        restored.remove_tab(d);
        restore_layout(&mut restored, layout);
        let expected = vec![(files(&["a"]), file("a")), (files(&["b", "c"]), file("c"))];
        assert_eq!(editors(&restored), expected);
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
