use std::sync::Arc;

use crate::tree::{Inode, Leaf, Metric, Node, Tree, TreeSlice};

/// An iterator over consecutive units of a particular metric.
pub struct Units<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> {
    /// TODO: docs
    forward: UnitsForward<'a, FANOUT, L, M>,

    /// TODO: docs
    backward: UnitsBackward<'a, FANOUT, L, M>,

    /// TODO: docs
    yielded: L::BaseMetric,

    /// TODO: docs
    total: L::BaseMetric,
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> Clone
    for Units<'a, FANOUT, L, M>
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            forward: self.forward.clone(),
            backward: self.backward.clone(),
            ..*self
        }
    }
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> From<&'a Tree<FANOUT, L>>
    for Units<'a, FANOUT, L, M>
where
    for<'d> &'d L::Slice: Default,
{
    #[inline]
    fn from(tree: &'a Tree<FANOUT, L>) -> Units<'a, FANOUT, L, M> {
        Self {
            forward: UnitsForward::new(tree.root(), None),
            backward: UnitsBackward::new(tree.root(), None),
            yielded: L::BaseMetric::zero(),
            total: L::BaseMetric::measure(tree.summary()),
        }
    }
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>>
    From<&'a TreeSlice<'a, FANOUT, L>> for Units<'a, FANOUT, L, M>
where
    for<'d> &'d L::Slice: Default,
{
    #[inline]
    fn from(
        tree_slice: &'a TreeSlice<'a, FANOUT, L>,
    ) -> Units<'a, FANOUT, L, M> {
        let base_total = L::BaseMetric::measure(tree_slice.summary());

        let units_total = M::measure(tree_slice.summary());

        let first_slice = (tree_slice.start_slice, &tree_slice.start_summary);

        let last_slice = (tree_slice.end_slice, &tree_slice.end_summary);

        let forward = UnitsForward::new(
            tree_slice.root(),
            Some((
                tree_slice.before,
                base_total,
                units_total,
                first_slice,
                last_slice,
            )),
        );

        let backward = UnitsBackward::new(
            tree_slice.root(),
            Some((
                tree_slice.before,
                base_total,
                units_total,
                first_slice,
                last_slice,
            )),
        );

        Self {
            forward,
            backward,
            yielded: L::BaseMetric::zero(),
            total: L::BaseMetric::measure(tree_slice.summary()),
        }
    }
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> Iterator
    for Units<'a, FANOUT, L, M>
{
    type Item = TreeSlice<'a, FANOUT, L>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.yielded == self.total {
            None
        } else {
            let tree_slice = self.forward.next()?;
            self.yielded += L::BaseMetric::measure(tree_slice.summary());
            Some(tree_slice)
        }
    }
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> DoubleEndedIterator
    for Units<'a, FANOUT, L, M>
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.yielded == self.total {
            None
        } else {
            let tree_slice = self.backward.next();
            self.yielded += L::BaseMetric::measure(tree_slice.summary());
            Some(tree_slice)
        }
    }
}

impl<'a, const FANOUT: usize, L: Leaf, M: Metric<L>> std::iter::FusedIterator
    for Units<'a, FANOUT, L, M>
{
}

#[derive(Debug)]
struct UnitsForward<'a, const N: usize, L: Leaf, M: Metric<L>> {
    /// TODO: docs
    is_initialized: bool,

    /// All the nodes in the stack are guaranteed to be internal nodes.
    stack: Vec<(&'a Arc<Node<N, L>>, usize)>,

    /// Guaranteed to be a leaf.
    leaf_node: &'a Arc<Node<N, L>>,

    /// TODO: docs,
    yielded_in_leaf: L::Summary,

    /// TODO: docs
    start_slice: &'a L::Slice,

    /// TODO: docs
    start_summary: L::Summary,

    /// TODO: docs
    first_slice: Option<(&'a L::Slice, &'a L::Summary)>,

    /// TODO: docs
    last_slice: Option<(&'a L::Slice, &'a L::Summary)>,

    /// TODO: docs
    base_offset: L::BaseMetric,

    /// TODO: docs
    base_yielded: L::BaseMetric,

    /// TODO: docs
    base_total: L::BaseMetric,

    /// TODO: docs
    units_yielded: M,

    /// TODO: docs
    units_total: M,
}

impl<'a, const N: usize, L: Leaf, M: Metric<L>> Clone
    for UnitsForward<'a, N, L, M>
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            stack: self.stack.clone(),
            yielded_in_leaf: self.yielded_in_leaf.clone(),
            start_summary: self.start_summary.clone(),
            ..*self
        }
    }
}

impl<'a, const N: usize, L: Leaf, M: Metric<L>> UnitsForward<'a, N, L, M> {
    #[inline]
    fn new(
        root: &'a Arc<Node<N, L>>,
        opt: Option<(
            L::BaseMetric,
            L::BaseMetric,
            M,
            (&'a L::Slice, &'a L::Summary),
            (&'a L::Slice, &'a L::Summary),
        )>,
    ) -> Self
    where
        for<'d> &'d L::Slice: Default,
    {
        let (base_offset, base_total, units_total, first_slice, last_slice) =
            match opt {
                Some((base_offset, base_total, units_total, start, end)) => (
                    base_offset,
                    base_total,
                    units_total,
                    Some(start),
                    Some(end),
                ),

                None => (
                    L::BaseMetric::zero(),
                    L::BaseMetric::measure(root.summary()),
                    M::measure(root.summary()),
                    None,
                    None,
                ),
            };

        Self {
            is_initialized: false,
            stack: Vec::with_capacity(root.depth()),
            leaf_node: root,
            yielded_in_leaf: L::Summary::default(),
            start_slice: <&L::Slice>::default(),
            start_summary: L::Summary::default(),
            first_slice,
            last_slice,
            base_offset,
            base_total,
            base_yielded: L::BaseMetric::zero(),
            units_total,
            units_yielded: M::zero(),
        }
    }

    /// TODO: docs
    #[inline]
    fn initialize(&mut self) {
        debug_assert!(!self.is_initialized);

        self.is_initialized = true;

        let mut measured = L::BaseMetric::zero();

        // The leaf node is actually the root at this point.
        let mut node = self.leaf_node;

        'outer: loop {
            match &**node {
                Node::Internal(inode) => {
                    for (idx, child) in inode.children().iter().enumerate() {
                        let child_measure =
                            L::BaseMetric::measure(child.summary());

                        if measured + child_measure > self.base_offset {
                            self.stack.push((node, idx));
                            node = child;
                            continue 'outer;
                        } else {
                            measured += child_measure;
                        }
                    }

                    unreachable!();
                },

                Node::Leaf(leaf) => {
                    self.leaf_node = node;

                    match self.first_slice.take() {
                        Some((slice, summary)) => {
                            self.yielded_in_leaf = leaf.summary().clone();
                            self.yielded_in_leaf -= summary;
                            self.start_slice = slice;
                            self.start_summary = summary.clone();
                        },

                        None => {
                            self.start_slice = leaf.as_slice();
                            self.start_summary = leaf.summary().clone();
                        },
                    }

                    return;
                },
            }
        }
    }

    /// Returns the last node in the stack together with a mutable reference to
    /// its current child index.
    #[inline]
    fn last_mut(&mut self) -> (&'a Arc<Node<N, L>>, &mut usize) {
        debug_assert!(!self.stack.is_empty());
        let last_idx = self.stack.len() - 1;
        let &mut (root, ref mut child_idx) = &mut self.stack[last_idx];
        (root, child_idx)
    }

    /// Like [`last_mut`](Self::last_mut), except the node is returned as an
    /// internal node.
    #[inline]
    fn last_as_internal_mut(&mut self) -> (&'a Inode<N, L>, &mut usize) {
        let (last, child_idx) = self.last_mut();
        // Safety: every node in the stack is an internal node.
        (unsafe { last.as_internal_unchecked() }, child_idx)
    }

    /// TODO: docs
    #[inline]
    fn next_leaf(&mut self) -> (&'a L::Slice, L::Summary) {
        debug_assert!(self.base_yielded < self.base_total);

        let mut node = loop {
            let (inode, child_idx) = self.last_as_internal_mut();

            *child_idx += 1;

            if *child_idx < inode.children().len() {
                break &inode.children()[*child_idx];
            } else if self.stack.len() > 1 {
                self.stack.pop();
            } else {
                // If we get here it means there's not a next leaf, but
                // `base_yielded + consider_extra_yielded < base_total`, so
                // there must be a next leaf.
                unreachable!();
            }
        };

        loop {
            match &**node {
                Node::Internal(inode) => {
                    self.stack.push((node, 0));
                    node = inode.first();
                    continue;
                },

                Node::Leaf(leaf) => {
                    self.leaf_node = node;

                    let (slice, summary) = if self.base_yielded
                        + L::BaseMetric::measure(leaf.summary())
                        <= self.base_total
                    {
                        (leaf.as_slice(), leaf.summary().clone())
                    } else {
                        // TODO: explain why we can unwrap here
                        let (slice, summary) = self.last_slice.take().unwrap();
                        (slice, summary.clone())
                    };

                    return (slice, summary);
                },
            }
        }
    }

    /// TODO: docs
    #[inline]
    fn next_unit_in_leaf(&mut self) -> TreeSlice<'a, N, L> {
        debug_assert!(M::measure(&self.start_summary) > M::zero());
        debug_assert!(self.units_yielded < self.units_total);

        let yielded = L::BaseMetric::measure(&self.yielded_in_leaf);

        let (start_slice, start_summary, rest_slice, rest_summary) =
            M::split(self.start_slice, M::one(), &self.start_summary);

        self.yielded_in_leaf += &start_summary;
        self.start_slice = rest_slice;
        self.start_summary = rest_summary;

        self.base_yielded += L::BaseMetric::measure(&start_summary);
        self.units_yielded += M::one();

        TreeSlice {
            root: self.leaf_node,
            before: yielded,
            summary: start_summary.clone(),
            end_slice: start_slice,
            end_summary: start_summary.clone(),
            start_slice,
            start_summary,
            num_leaves: 1,
        }
    }

    /// TODO: docs
    #[inline]
    fn next_unit_in_stack(&mut self) -> TreeSlice<'a, N, L> {
        debug_assert_eq!(M::measure(&self.start_summary), M::zero());
        debug_assert!(self.units_yielded < self.units_total);

        // A previous call to `next_unit_in_leaf` might've left the start slice
        // empty.
        if L::BaseMetric::measure(&self.start_summary) == L::BaseMetric::zero()
        {
            let (next_slice, next_summary) = self.next_leaf();
            self.yielded_in_leaf = L::Summary::default();
            self.start_slice = next_slice;
            self.start_summary = next_summary;

            if M::measure(&self.start_summary) > M::zero() {
                return self.next_unit_in_leaf();
            }
        }

        let start_slice = self.start_slice;
        let start_summary = self.start_summary.clone();

        let mut yielded = self.yielded_in_leaf.clone();
        let mut summary = start_summary.clone();
        let mut num_leaves = 1;

        // 1: find the root.

        'outer: loop {
            let (node, mut child_idx) = self.stack.pop().unwrap();

            // Safety: every node in the stack is an internal node.
            let inode = unsafe { node.as_internal_unchecked() };

            for child in &inode.children()[..child_idx] {
                yielded += child.summary();
            }

            if M::measure(inode.summary()) > M::measure(&yielded) {
                // This is the root and it needs to be pushed back onto the
                // stack.

                child_idx += 1;

                for child in &inode.children()[child_idx..] {
                    if M::measure(child.summary()) > M::zero() {
                        self.stack.push((node, child_idx));
                        break 'outer;
                    } else {
                        child_idx += 1;
                        summary += child.summary();
                        num_leaves += child.num_leaves();
                    }
                }

                unreachable!();
            } else {
                for child in &inode.children()[child_idx + 1..] {
                    summary += child.summary();
                    num_leaves += child.num_leaves();
                }
            }
        }

        // 2.

        let (root, child_idx) = self.last_mut();

        // Safety: every node in the stack is an internal node.
        let mut node =
            unsafe { &root.as_internal_unchecked().children()[*child_idx] };

        let (leaf_slice, leaf_summary) = 'outer: loop {
            match &**node {
                Node::Internal(inode) => {
                    for (idx, child) in inode.children().iter().enumerate() {
                        if M::measure(child.summary()) != M::zero() {
                            self.stack.push((node, idx));
                            node = child;
                            continue 'outer;
                        } else {
                            summary += child.summary();
                            num_leaves += child.num_leaves();
                        }
                    }
                },

                Node::Leaf(leaf) => {
                    self.leaf_node = node;

                    let (slice, summary) = if self.base_yielded
                        + L::BaseMetric::measure(&summary)
                        + L::BaseMetric::measure(leaf.summary())
                        <= self.base_total
                    {
                        (leaf.as_slice(), leaf.summary())
                    } else {
                        self.last_slice.take().unwrap()
                    };

                    break (slice, summary);
                },
            }
        };

        debug_assert!(M::measure(leaf_summary) >= M::one());

        // 3.

        let (end_slice, end_summary, rest_slice, rest_summary) =
            M::split(leaf_slice, M::one(), leaf_summary);

        summary += &end_summary;
        num_leaves += 1;

        self.base_yielded += L::BaseMetric::measure(&summary);
        self.units_yielded += M::one();

        if L::BaseMetric::measure(&rest_summary) > L::BaseMetric::zero() {
            self.yielded_in_leaf = end_summary.clone();
            self.start_slice = rest_slice;
            self.start_summary = rest_summary;
        } else if self.base_yielded < self.base_total {
            let (next_slice, next_summary) = self.next_leaf();
            self.yielded_in_leaf = L::Summary::default();
            self.start_slice = next_slice;
            self.start_summary = next_summary;
        }

        TreeSlice {
            root,
            before: L::BaseMetric::measure(&yielded),
            summary,
            end_slice,
            end_summary,
            start_slice,
            start_summary,
            num_leaves,
        }
    }

    /// TODO: docs
    #[inline]
    fn yield_remaining(&mut self) -> TreeSlice<'a, N, L> {
        debug_assert_eq!(self.units_yielded, self.units_total);
        debug_assert!(self.base_yielded < self.base_total);

        let (mut yielded, start_slice, start_summary) =
            if L::BaseMetric::measure(&self.start_summary)
                == L::BaseMetric::zero()
            {
                let (next_slice, next_summary) = self.next_leaf();
                (L::BaseMetric::zero(), next_slice, next_summary)
            } else {
                (
                    L::BaseMetric::measure(&self.yielded_in_leaf),
                    self.start_slice,
                    std::mem::take(&mut self.start_summary),
                )
            };

        let mut summary = start_summary.clone();
        let mut num_leaves = 1;

        // First, check if the leaf node is the root. If it is we're done.
        if self.base_yielded + L::BaseMetric::measure(&start_summary)
            == self.base_total
        {
            return TreeSlice {
                root: self.leaf_node,
                before: yielded,
                summary,
                end_slice: start_slice,
                end_summary: start_summary.clone(),
                start_slice,
                start_summary,
                num_leaves,
            };
        }

        // 1: find the root in the stack.
        //
        // The root is the deepest node in the stack that fully contains the
        // `(base_offset + base_yielded)..(base_offset + base_total)` range.

        let mut range = (self.base_offset + self.base_yielded)
            ..(self.base_offset + self.base_total);

        let root_idx = {
            let mut root_idx = self.stack.len() - 1;

            'outer: for (stack_idx, &(node, child_idx)) in
                self.stack.iter().enumerate()
            {
                // Safety: every node in the stack is an internal node.
                let inode = unsafe { node.as_internal_unchecked() };

                let mut measured = L::BaseMetric::zero();

                for child in &inode.children()[..child_idx] {
                    measured += child.base_measure();
                }

                for child in &inode.children()[child_idx..] {
                    let child_measure = child.base_measure();

                    if measured <= range.start
                        && measured + child_measure >= range.end
                    {
                        range.start -= measured;
                        range.end -= measured;
                        continue 'outer;
                    } else {
                        measured += child_measure;
                    }
                }

                root_idx = stack_idx;
                break;
            }

            root_idx
        };

        //

        // Keep the old code to increase `summary`, `num_leaves`, `yielded`,
        // except it starts from `root_idx + 1` instead of `root_idx`.
        //
        // At the root_idx level do the same until `child_idx`, skip
        // `child_idx`, then check which child contain the end_slice from
        // `(child_idx + 1)..`.

        // 2: increase `yielded`, `summary`, `num_leaves`.

        for &(node, child_idx) in &self.stack[root_idx + 1..] {
            // Safety: every node in the stack is an internal node.
            let inode = unsafe { node.as_internal_unchecked() };

            for child in &inode.children()[..child_idx] {
                yielded += child.base_measure();
            }

            for child in &inode.children()[child_idx + 1..] {
                summary += child.summary();
                num_leaves += child.num_leaves();
            }
        }

        let (root, child_idx) = self.stack[root_idx];

        // Safety: every node in the stack is an internal node.
        let inode = unsafe { root.as_internal_unchecked() };

        let mut measured = L::BaseMetric::zero();

        for child in &inode.children()[..child_idx] {
            let child_measure = child.base_measure();
            measured += child_measure;
            yielded += child_measure;
        }

        measured += inode.children()[child_idx].base_measure();

        let mut node = inode.first();

        for child in &inode.children()[child_idx + 1..] {
            let child_measure = child.base_measure();

            if measured + child_measure >= range.end {
                node = child;
                break;
            } else {
                summary += child.summary();
                num_leaves += child.num_leaves();
                measured += child_measure;
            }
        }

        let (end_slice, end_summary) = 'outer: loop {
            match &**node {
                Node::Internal(inode) => {
                    for child in inode.children() {
                        let child_measure = child.base_measure();

                        if measured + child_measure >= range.end {
                            node = child;
                            continue 'outer;
                        } else {
                            summary += child.summary();
                            num_leaves += child.num_leaves();
                            measured += child_measure;
                        }
                    }

                    unreachable!();
                },

                Node::Leaf(leaf) => {
                    break (match self.last_slice.take() {
                        Some(last) => last,
                        None => (leaf.as_slice(), leaf.summary()),
                    })
                },
            }
        };

        summary += end_summary;
        num_leaves += 1;

        self.base_yielded += L::BaseMetric::measure(&summary);

        debug_assert_eq!(self.base_yielded, self.base_total);

        TreeSlice {
            root,
            before: yielded,
            summary,
            start_slice,
            start_summary,
            num_leaves,
            end_slice,
            end_summary: end_summary.clone(),
        }
    }

    #[inline]
    fn next(&mut self) -> Option<TreeSlice<'a, N, L>> {
        if !self.is_initialized {
            self.initialize();
        }

        if M::measure(&self.start_summary) > M::zero() {
            Some(self.next_unit_in_leaf())
        } else if self.units_yielded < self.units_total {
            Some(self.next_unit_in_stack())
        } else if self.base_yielded < self.base_total {
            Some(self.yield_remaining())
        } else {
            None
        }
    }
}

#[derive(Debug)]
struct UnitsBackward<'a, const N: usize, L: Leaf, M: Metric<L>> {
    /// TODO: docs
    is_initialized: bool,

    /// All the nodes in the stack are guaranteed to be internal nodes.
    stack: Vec<(&'a Arc<Node<N, L>>, usize)>,

    /// Guaranteed to be a leaf.
    leaf_node: &'a Arc<Node<N, L>>,

    /// TODO: docs,
    yielded_in_leaf: L::Summary,

    /// TODO: docs
    end_slice: &'a L::Slice,

    /// TODO: docs
    end_summary: L::Summary,

    /// TODO: docs
    first_slice: Option<(&'a L::Slice, &'a L::Summary)>,

    /// TODO: docs
    last_slice: Option<(&'a L::Slice, &'a L::Summary)>,

    /// TODO: docs
    base_offset: L::BaseMetric,

    /// TODO: docs
    base_yielded: L::BaseMetric,

    /// TODO: docs
    base_total: L::BaseMetric,

    /// TODO: docs
    units_yielded: M,

    /// TODO: docs
    units_total: M,
}

impl<'a, const N: usize, L: Leaf, M: Metric<L>> Clone
    for UnitsBackward<'a, N, L, M>
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            stack: self.stack.clone(),
            yielded_in_leaf: self.yielded_in_leaf.clone(),
            end_summary: self.end_summary.clone(),
            ..*self
        }
    }
}

impl<'a, const N: usize, L: Leaf, M: Metric<L>> UnitsBackward<'a, N, L, M> {
    #[inline]
    fn new(
        root: &'a Arc<Node<N, L>>,
        opt: Option<(
            L::BaseMetric,
            L::BaseMetric,
            M,
            (&'a L::Slice, &'a L::Summary),
            (&'a L::Slice, &'a L::Summary),
        )>,
    ) -> Self
    where
        for<'d> &'d L::Slice: Default,
    {
        let (base_offset, base_total, units_total, first_slice, last_slice) =
            match opt {
                Some((base_offset, base_total, units_total, start, end)) => (
                    base_offset,
                    base_total,
                    units_total,
                    Some(start),
                    Some(end),
                ),

                None => (
                    L::BaseMetric::zero(),
                    L::BaseMetric::measure(root.summary()),
                    M::measure(root.summary()),
                    None,
                    None,
                ),
            };

        Self {
            is_initialized: false,
            stack: Vec::with_capacity(root.depth()),
            leaf_node: root,
            yielded_in_leaf: L::Summary::default(),
            end_slice: <&L::Slice>::default(),
            end_summary: L::Summary::default(),
            first_slice,
            last_slice,
            base_offset,
            base_total,
            base_yielded: L::BaseMetric::zero(),
            units_total,
            units_yielded: M::zero(),
        }
    }

    #[inline]
    fn initialize(&mut self) {}

    #[inline]
    fn next(&mut self) -> TreeSlice<'a, N, L> {
        if !self.is_initialized {
            self.initialize();
        }

        debug_assert!(
            L::BaseMetric::measure(&self.end_summary) > L::BaseMetric::zero()
        );

        todo!();
    }
}
