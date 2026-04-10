//! Accessibility tree construction and diffing.
//!
//! During each render pass, widgets push [`A11yNodeInfo`] into an
//! [`A11yTreeBuilder`]. At the end of the pass the builder is consumed
//! to produce an immutable [`A11yTree`] snapshot. Consecutive snapshots
//! can be diffed to produce an [`A11yTreeDiff`] describing exactly what
//! changed -- this is the data a platform accessibility bridge would
//! push to the OS.

use ahash::AHashMap;

use crate::node::{A11yNodeInfo, A11yRole, LiveRegion};

// ── Builder ────────────────────────────────────────────────────────────

/// Accumulates accessibility nodes during a render pass.
///
/// Usage:
/// 1. Create with [`A11yTreeBuilder::new`].
/// 2. Add nodes via [`add_node`](Self::add_node).
/// 3. Optionally designate the root and focused node.
/// 4. Call [`build`](Self::build) to freeze.
pub struct A11yTreeBuilder {
    nodes: AHashMap<u64, A11yNodeInfo>,
    root: Option<u64>,
    focused: Option<u64>,
}

impl Default for A11yTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl A11yTreeBuilder {
    /// Create an empty builder.
    #[inline]
    pub fn new() -> Self {
        Self {
            nodes: AHashMap::new(),
            root: None,
            focused: None,
        }
    }

    /// Create a builder pre-sized for `capacity` nodes (avoids reallocs).
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: AHashMap::with_capacity(capacity),
            root: None,
            focused: None,
        }
    }

    /// Insert or replace a node.
    #[inline]
    pub fn add_node(&mut self, node: A11yNodeInfo) {
        self.nodes.insert(node.id, node);
    }

    /// Designate the root node ID.
    #[inline]
    pub fn set_root(&mut self, id: u64) {
        self.root = Some(id);
    }

    /// Designate the currently focused node.
    #[inline]
    pub fn set_focused(&mut self, id: Option<u64>) {
        self.focused = id;
    }

    /// Consume the builder and produce an immutable tree snapshot.
    #[inline]
    pub fn build(self) -> A11yTree {
        A11yTree {
            nodes: self.nodes,
            root: self.root,
            focused: self.focused,
        }
    }
}

// ── Immutable tree ─────────────────────────────────────────────────────

/// Immutable snapshot of the accessibility tree after a render pass.
///
/// The tree is a flat map keyed by node ID; parent/child relationships
/// are encoded inside each [`A11yNodeInfo`]. This makes traversal O(1)
/// per hop and diffing O(n) in the number of nodes.
pub struct A11yTree {
    nodes: AHashMap<u64, A11yNodeInfo>,
    root: Option<u64>,
    focused: Option<u64>,
}

impl Default for A11yTree {
    fn default() -> Self {
        Self::empty()
    }
}

impl A11yTree {
    /// Create an empty tree (no nodes, no root, no focus).
    #[inline]
    pub fn empty() -> Self {
        Self {
            nodes: AHashMap::new(),
            root: None,
            focused: None,
        }
    }

    /// Look up a node by ID.
    #[inline]
    pub fn node(&self, id: u64) -> Option<&A11yNodeInfo> {
        self.nodes.get(&id)
    }

    /// The root node, if set and present.
    #[inline]
    pub fn root(&self) -> Option<&A11yNodeInfo> {
        self.root.and_then(|id| self.nodes.get(&id))
    }

    /// The root node ID, if set.
    #[inline]
    pub fn root_id(&self) -> Option<u64> {
        self.root
    }

    /// The focused node, if set and present.
    #[inline]
    pub fn focused(&self) -> Option<&A11yNodeInfo> {
        self.focused.and_then(|id| self.nodes.get(&id))
    }

    /// The focused node ID, if set.
    #[inline]
    pub fn focused_id(&self) -> Option<u64> {
        self.focused
    }

    /// Iterate over all nodes in unspecified order.
    #[inline]
    pub fn nodes(&self) -> impl Iterator<Item = &A11yNodeInfo> {
        self.nodes.values()
    }

    /// Total number of nodes.
    #[inline]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree contains zero nodes.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get all children of a given node ID, in the order stored.
    pub fn children_of(&self, id: u64) -> Vec<&A11yNodeInfo> {
        self.nodes
            .get(&id)
            .map(|n| {
                n.children
                    .iter()
                    .filter_map(|cid| self.nodes.get(cid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Walk ancestors from `id` up to the root (inclusive), returning
    /// their IDs. Returns an empty vec if `id` is not found.
    ///
    /// Includes cycle protection: stops after visiting 1000 nodes to
    /// prevent infinite loops on malformed cyclic parent chains.
    pub fn ancestors(&self, id: u64) -> Vec<u64> {
        const MAX_DEPTH: usize = 1000;
        let mut path = Vec::new();
        let mut visited = ahash::AHashSet::new();
        let mut current = Some(id);
        while let Some(cid) = current {
            if path.len() >= MAX_DEPTH || !visited.insert(cid) {
                break;
            }
            if let Some(node) = self.nodes.get(&cid) {
                path.push(cid);
                current = node.parent;
            } else {
                break;
            }
        }
        path
    }

    /// Diff this tree against a previous snapshot to find changes.
    ///
    /// Returns an [`A11yTreeDiff`] describing additions, removals,
    /// property changes, and focus transitions.
    pub fn diff(&self, previous: &A11yTree) -> A11yTreeDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        // Nodes in self but not in previous => added.
        // Nodes in both => check for property changes.
        for (&id, node) in &self.nodes {
            match previous.nodes.get(&id) {
                None => added.push(id),
                Some(old) => {
                    let changes = diff_node(old, node);
                    if !changes.is_empty() {
                        changed.push((id, changes));
                    }
                }
            }
        }

        // Nodes in previous but not in self => removed.
        for &id in previous.nodes.keys() {
            if !self.nodes.contains_key(&id) {
                removed.push(id);
            }
        }

        let focus_changed = if self.focused != previous.focused {
            Some((previous.focused, self.focused))
        } else {
            None
        };

        A11yTreeDiff {
            added,
            removed,
            changed,
            focus_changed,
        }
    }
}

// ── Diff types ─────────────────────────────────────────────────────────

/// Changes between two accessibility tree snapshots.
///
/// Produced by [`A11yTree::diff`]. A platform bridge would translate
/// these into OS accessibility events.
#[derive(Debug, Clone)]
pub struct A11yTreeDiff {
    /// Node IDs that are new in the current tree.
    pub added: Vec<u64>,
    /// Node IDs that were in the previous tree but are gone now.
    pub removed: Vec<u64>,
    /// Node IDs whose properties changed, with details.
    pub changed: Vec<(u64, Vec<A11yChange>)>,
    /// Focus transition: `Some((old_focus, new_focus))`.
    /// Either side may be `None` if focus was gained/lost entirely.
    pub focus_changed: Option<(Option<u64>, Option<u64>)>,
}

impl A11yTreeDiff {
    /// Returns `true` if nothing changed between the two snapshots.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed.is_empty()
            && self.focus_changed.is_none()
    }
}

/// A single property change on an accessibility node.
#[derive(Debug, Clone, PartialEq)]
pub enum A11yChange {
    /// The accessible name changed.
    NameChanged {
        old: Option<String>,
        new: Option<String>,
    },
    /// The role changed (unusual but possible during dynamic UIs).
    RoleChanged { old: A11yRole, new: A11yRole },
    /// A state flag changed. `field` is the flag name, `description`
    /// is a human-readable summary of the new value.
    StateChanged { field: String, description: String },
    /// The bounding rectangle moved or resized.
    BoundsChanged,
    /// The set of child IDs changed.
    ChildrenChanged,
    /// The live-region policy changed.
    LiveRegionChanged {
        old: Option<LiveRegion>,
        new: Option<LiveRegion>,
    },
    /// The accessible description changed.
    DescriptionChanged {
        old: Option<String>,
        new: Option<String>,
    },
    /// The keyboard shortcut hint changed.
    ShortcutChanged {
        old: Option<String>,
        new: Option<String>,
    },
    /// The parent node ID changed.
    ParentChanged { old: Option<u64>, new: Option<u64> },
}

// ── Internal diff helpers ──────────────────────────────────────────────

fn diff_node(old: &A11yNodeInfo, new: &A11yNodeInfo) -> Vec<A11yChange> {
    let mut changes = Vec::new();

    if old.name != new.name {
        changes.push(A11yChange::NameChanged {
            old: old.name.clone(),
            new: new.name.clone(),
        });
    }

    if old.role != new.role {
        changes.push(A11yChange::RoleChanged {
            old: old.role,
            new: new.role,
        });
    }

    if old.bounds != new.bounds {
        changes.push(A11yChange::BoundsChanged);
    }

    if old.children != new.children {
        changes.push(A11yChange::ChildrenChanged);
    }

    if old.live_region != new.live_region {
        changes.push(A11yChange::LiveRegionChanged {
            old: old.live_region,
            new: new.live_region,
        });
    }

    if old.description != new.description {
        changes.push(A11yChange::DescriptionChanged {
            old: old.description.clone(),
            new: new.description.clone(),
        });
    }

    if old.shortcut != new.shortcut {
        changes.push(A11yChange::ShortcutChanged {
            old: old.shortcut.clone(),
            new: new.shortcut.clone(),
        });
    }

    if old.parent != new.parent {
        changes.push(A11yChange::ParentChanged {
            old: old.parent,
            new: new.parent,
        });
    }

    // Diff individual state fields.
    diff_state(&old.state, &new.state, &mut changes);

    changes
}

fn diff_state(
    old: &crate::node::A11yState,
    new: &crate::node::A11yState,
    changes: &mut Vec<A11yChange>,
) {
    macro_rules! check_bool {
        ($field:ident) => {
            if old.$field != new.$field {
                changes.push(A11yChange::StateChanged {
                    field: stringify!($field).to_owned(),
                    description: new.$field.to_string(),
                });
            }
        };
    }

    macro_rules! check_option {
        ($field:ident) => {
            if old.$field != new.$field {
                changes.push(A11yChange::StateChanged {
                    field: stringify!($field).to_owned(),
                    description: format!("{:?}", new.$field),
                });
            }
        };
    }

    check_bool!(focused);
    check_bool!(disabled);
    check_option!(checked);
    check_option!(expanded);
    check_bool!(selected);
    check_bool!(readonly);
    check_bool!(required);
    check_bool!(busy);
    check_option!(value_now);
    check_option!(value_min);
    check_option!(value_max);

    if old.value_text != new.value_text {
        changes.push(A11yChange::StateChanged {
            field: "value_text".to_owned(),
            description: new.value_text.as_deref().unwrap_or("<none>").to_owned(),
        });
    }
}
