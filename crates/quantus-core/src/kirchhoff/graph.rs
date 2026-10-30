//! The drawing as a graph: grid points joined by parts and wires, turned into electrical nodes and
//! the loops a student would pick.
//!
//! The loops are the windows of the drawing, its bounded faces, each walked clockwise on the
//! screen (y points down). That needs a planar drawing: if two segments cross, overlap or a
//! terminal sits in the middle of another segment, the loops come from a spanning tree instead,
//! one per link, and [`Graph::planar`] is false.
//!
//! Ideal wires merge the points they join into one electrical node and drop out of the loops. All
//! geometry is exact integer arithmetic on grid coordinates.

use std::cmp::Ordering;
use std::collections::HashMap;

use dashu::rational::RBig;

use crate::electro::text::msg;
use crate::{QuantusError, Result};

/// A grid point, in grid steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub fn new(x: i64, y: i64) -> Self {
        Point { x, y }
    }

    /// Reading order: top to bottom, then left to right.
    fn key(self) -> (i64, i64) {
        (self.y, self.x)
    }
}

/// A part or a wire of the drawing, a straight segment from terminal `a` to terminal `b`.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: String,
    pub a: Point,
    pub b: Point,
    pub wire: bool,
}

/// An electrical node: grid points joined by wires.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// In reading order, so the first one is the node's place in the drawing.
    pub points: Vec<Point>,
    /// The connected part of the circuit.
    pub part: usize,
}

/// A part that is not a wire, between two electrical nodes.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    /// Index into the edges the graph was built from.
    pub edge: usize,
    pub a: usize,
    pub b: usize,
}

/// One link of a loop, `forward` if the loop runs through it from `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub link: usize,
    pub forward: bool,
}

/// Where a loop's name and arrow go, in grid steps; may lie halfway between grid points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Loop {
    pub steps: Vec<Step>,
    pub anchor: Anchor,
    pub part: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graph {
    /// In reading order of their first point.
    pub nodes: Vec<Node>,
    /// In the order of the edges.
    pub links: Vec<Link>,
    /// Independent loops in reading order of their anchors, as many as the circuit needs.
    pub loops: Vec<Loop>,
    /// Whether the loops are the windows of the drawing rather than spanning-tree loops.
    pub planar: bool,
    pub parts: usize,
}

impl Graph {
    pub fn build(edges: &[Edge]) -> Result<Graph> {
        let mut points: Vec<Point> = Vec::new();
        let mut index: HashMap<Point, usize> = HashMap::new();
        let mut vertex = |p: Point| {
            *index.entry(p).or_insert_with(|| {
                points.push(p);
                points.len() - 1
            })
        };
        let ends: Vec<(usize, usize)> = edges.iter().map(|e| (vertex(e.a), vertex(e.b))).collect();

        // Wires merge their end points.
        let mut joined = UnionFind::new(points.len());
        for (edge, &(a, b)) in edges.iter().zip(&ends) {
            if edge.wire {
                joined.union(a, b);
            }
        }
        let mut roots: Vec<usize> = Vec::new();
        for (edge, &(a, b)) in edges.iter().zip(&ends) {
            if !edge.wire {
                roots.extend([joined.find(a), joined.find(b)]);
            }
        }
        roots.sort_unstable();
        roots.dedup();
        let mut nodes: Vec<(usize, Node)> = roots
            .iter()
            .map(|&root| {
                let mut members: Vec<Point> =
                    (0..points.len()).filter(|&v| joined.find(v) == root).map(|v| points[v]).collect();
                members.sort_by_key(|p| p.key());
                (root, Node { points: members, part: 0 })
            })
            .collect();
        nodes.sort_by_key(|(_, node)| node.points[0].key());
        let node_of_root: HashMap<usize, usize> = nodes.iter().enumerate().map(|(n, (root, _))| (*root, n)).collect();
        let mut nodes: Vec<Node> = nodes.into_iter().map(|(_, node)| node).collect();

        let mut links = Vec::new();
        let mut link_of_edge = vec![None; edges.len()];
        for (n, (edge, &(a, b))) in edges.iter().zip(&ends).enumerate() {
            if !edge.wire {
                link_of_edge[n] = Some(links.len());
                links.push(Link { edge: n, a: node_of_root[&joined.find(a)], b: node_of_root[&joined.find(b)] });
            }
        }

        let mut degree = vec![0usize; nodes.len()];
        for link in &links {
            degree[link.a] += 1;
            degree[link.b] += 1;
        }
        if let Some(open) = (0..nodes.len()).find(|&n| degree[n] == 1) {
            let link = links.iter().find(|l| l.a == open || l.b == open).expect("a node of degree 1 has a link");
            let id = &edges[link.edge].id;
            return Err(QuantusError::other(msg!(
                "{id} has an open end: nothing is connected to one of its terminals.",
                "{id} hat ein offenes Ende: an einem Anschluss ist nichts angeschlossen."
            )));
        }

        let mut connected = UnionFind::new(nodes.len());
        for link in &links {
            connected.union(link.a, link.b);
        }
        let mut part_of_root = HashMap::new();
        for (n, node) in nodes.iter_mut().enumerate() {
            let next = part_of_root.len();
            node.part = *part_of_root.entry(connected.find(n)).or_insert(next);
        }
        let parts = part_of_root.len();

        let planar = is_planar(edges);
        let candidates = if planar {
            Faces::new(edges, &points, &ends).loops(edges, &link_of_edge)
        } else {
            tree_loops(edges, &nodes, &links)
        };
        let mut loops: Vec<Loop> = Vec::new();
        let mut basis = Basis::default();
        for (steps, anchor) in candidates {
            if basis.insert(&steps, links.len()) {
                let part = nodes[links[steps[0].link].a].part;
                loops.push(Loop { steps, anchor, part });
            }
        }
        debug_assert_eq!(loops.len() + nodes.len(), links.len() + parts);
        loops.sort_by(|p, q| (p.anchor.y, p.anchor.x).partial_cmp(&(q.anchor.y, q.anchor.x)).unwrap_or(Ordering::Equal));
        Ok(Graph { nodes, links, loops, planar, parts })
    }
}

struct UnionFind(Vec<usize>);

impl UnionFind {
    fn new(n: usize) -> Self {
        UnionFind((0..n).collect())
    }

    fn find(&mut self, mut v: usize) -> usize {
        while self.0[v] != v {
            self.0[v] = self.0[self.0[v]];
            v = self.0[v];
        }
        v
    }

    fn union(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a.max(b)] = a.min(b);
    }
}

fn cross(o: Point, p: Point, q: Point) -> i64 {
    (p.x - o.x) * (q.y - o.y) - (p.y - o.y) * (q.x - o.x)
}

/// Whether `p`, known to be on the line through `a` and `b`, lies on the segment.
fn within(a: Point, b: Point, p: Point) -> bool {
    a.x.min(b.x) <= p.x && p.x <= a.x.max(b.x) && a.y.min(b.y) <= p.y && p.y <= a.y.max(b.y)
}

/// Whether two segments meet anywhere but at an end point they share.
fn clash(p: (Point, Point), q: (Point, Point)) -> bool {
    let (d1, d2) = (cross(q.0, q.1, p.0), cross(q.0, q.1, p.1));
    let (d3, d4) = (cross(p.0, p.1, q.0), cross(p.0, p.1, q.1));
    if d1 == 0 && d2 == 0 {
        // Collinear: they overlap if their shadows on an axis share more than a point.
        let along = |t: Point| if p.0.x != p.1.x { t.x } else { t.y };
        let (p0, p1, q0, q1) = (along(p.0), along(p.1), along(q.0), along(q.1));
        return p0.max(p1).min(q0.max(q1)) > p0.min(p1).max(q0.min(q1));
    }
    let shared = [p.0, p.1].iter().any(|e| *e == q.0 || *e == q.1);
    if shared {
        return false;
    }
    (d1.signum() * d2.signum() < 0 && d3.signum() * d4.signum() < 0)
        || (d1 == 0 && within(q.0, q.1, p.0))
        || (d2 == 0 && within(q.0, q.1, p.1))
        || (d3 == 0 && within(p.0, p.1, q.0))
        || (d4 == 0 && within(p.0, p.1, q.1))
}

/// No two segments meet away from shared end points, and no part has length zero. Wires of
/// length zero are left out: they join nothing.
fn is_planar(edges: &[Edge]) -> bool {
    let segments: Vec<(Point, Point)> = edges.iter().filter(|e| e.a != e.b).map(|e| (e.a, e.b)).collect();
    if edges.iter().any(|e| !e.wire && e.a == e.b) {
        return false;
    }
    (0..segments.len()).all(|i| (i + 1..segments.len()).all(|j| !clash(segments[i], segments[j])))
}

/// The half-edges of a planar drawing: half-edge `2e` runs along edge `e` from `a` to `b`,
/// `2e + 1` back. Edges of length zero have none.
struct Faces {
    /// The boundary walk of each face, which lies on the right of its half-edges.
    walks: Vec<Vec<usize>>,
}

impl Faces {
    fn new(edges: &[Edge], points: &[Point], ends: &[(usize, usize)]) -> Self {
        let from = |h: usize| if h.is_multiple_of(2) { ends[h / 2].0 } else { ends[h / 2].1 };
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); points.len()];
        for (e, edge) in edges.iter().enumerate() {
            if edge.a != edge.b {
                outgoing[ends[e].0].push(2 * e);
                outgoing[ends[e].1].push(2 * e + 1);
            }
        }
        let direction = |h: usize| {
            let (o, t) = (points[from(h)], points[from(h ^ 1)]);
            Point::new(t.x - o.x, t.y - o.y)
        };
        // Counterclockwise in plain coordinates, starting along the positive x axis.
        let half = |d: Point| if d.y > 0 || d.y == 0 && d.x > 0 { 0 } else { 1 };
        let mut place = vec![0usize; 2 * edges.len()];
        for out in &mut outgoing {
            out.sort_by(|&g, &h| {
                let (d, e) = (direction(g), direction(h));
                half(d).cmp(&half(e)).then_with(|| cross(Point::new(0, 0), e, d).cmp(&0))
            });
            for (n, &h) in out.iter().enumerate() {
                place[h] = n;
            }
        }
        // Leaving a vertex by the next half-edge after the way back is the sharpest right
        // turn, so every face lies on the right of its half-edges.
        let next = |h: usize| {
            let out = &outgoing[from(h ^ 1)];
            out[(place[h ^ 1] + 1) % out.len()]
        };
        let mut seen = vec![false; 2 * edges.len()];
        let mut walks = Vec::new();
        for start in (0..2 * edges.len()).filter(|&h| edges[h / 2].a != edges[h / 2].b) {
            if seen[start] {
                continue;
            }
            let mut walk = Vec::new();
            let mut h = start;
            while !seen[h] {
                seen[h] = true;
                walk.push(h);
                h = next(h);
            }
            walks.push(walk);
        }
        Faces { walks }
    }

    /// The bounded faces as loops, clockwise on the screen, with their anchors. A face's wires drop
    /// out, and so does a part it passes both ways (a bridge into the face).
    fn loops(&self, edges: &[Edge], link_of_edge: &[Option<usize>]) -> Vec<(Vec<Step>, Anchor)> {
        let start = |h: usize| if h.is_multiple_of(2) { edges[h / 2].a } else { edges[h / 2].b };
        let mut loops = Vec::new();
        for walk in &self.walks {
            let area: i64 = walk.iter().map(|&h| cross(Point::new(0, 0), start(h), start(h ^ 1))).sum();
            // The face lies on the right: in plain coordinates a bounded face then turns
            // clockwise and has negative area. With y down that is counterclockwise on the
            // screen, so the loop runs the walk backwards.
            if area >= 0 {
                continue;
            }
            let back: Vec<usize> = walk.iter().rev().map(|&h| h ^ 1).collect();
            let mut steps: Vec<Step> = back
                .iter()
                .filter_map(|&h| link_of_edge[h / 2].map(|link| Step { link, forward: h.is_multiple_of(2) }))
                .collect();
            let twice: Vec<usize> =
                steps.iter().filter(|s| steps.iter().any(|t| t.link == s.link && t.forward != s.forward)).map(|s| s.link).collect();
            steps.retain(|s| !twice.contains(&s.link));
            if steps.is_empty() {
                continue;
            }
            let first = (0..steps.len()).min_by_key(|&n| steps[n].link).unwrap_or(0);
            steps.rotate_left(first);
            let polygon: Vec<Point> = back.iter().map(|&h| start(h)).collect();
            loops.push((steps, anchor(&polygon, edges)));
        }
        loops
    }
}

/// The point inside `polygon`, on a half-step grid, farthest from every segment of the drawing;
/// of equally far ones, the one nearest the middle of the polygon's bounding box.
fn anchor(polygon: &[Point], edges: &[Edge]) -> Anchor {
    let double = |p: Point| Point::new(2 * p.x, 2 * p.y);
    let corners: Vec<Point> = polygon.iter().map(|&p| double(p)).collect();
    let segments: Vec<(Point, Point)> = edges.iter().map(|e| (double(e.a), double(e.b))).collect();
    let (left, right) = (corners.iter().map(|p| p.x).min().unwrap_or(0), corners.iter().map(|p| p.x).max().unwrap_or(0));
    let (top, bottom) = (corners.iter().map(|p| p.y).min().unwrap_or(0), corners.iter().map(|p| p.y).max().unwrap_or(0));
    let middle = Point::new(left + right, top + bottom);
    let off_middle = |p: Point| (2 * p.x - middle.x).pow(2) + (2 * p.y - middle.y).pow(2);
    let mut best: Option<(f64, Point)> = None;
    for y in top + 1..bottom {
        for x in left + 1..right {
            let p = Point::new(x, y);
            if !inside(&corners, p) {
                continue;
            }
            let distance = segments.iter().map(|&(a, b)| distance(a, b, p)).fold(f64::INFINITY, f64::min);
            if best.is_none_or(|(d, q)| distance > d || distance == d && off_middle(p) < off_middle(q)) {
                best = Some((distance, p));
            }
        }
    }
    match best {
        Some((_, p)) => Anchor { x: p.x as f64 / 2.0, y: p.y as f64 / 2.0 },
        None => centroid(polygon.iter().copied()),
    }
}

fn centroid(points: impl Iterator<Item = Point>) -> Anchor {
    let (mut x, mut y, mut n) = (0.0, 0.0, 0.0);
    for p in points {
        x += p.x as f64;
        y += p.y as f64;
        n += 1.0;
    }
    if n == 0.0 {
        return Anchor { x: 0.0, y: 0.0 };
    }
    Anchor { x: x / n, y: y / n }
}

/// Strictly inside by the even-odd rule; a point on the outline is not.
fn inside(polygon: &[Point], p: Point) -> bool {
    let mut odd = false;
    for (n, &a) in polygon.iter().enumerate() {
        let b = polygon[(n + 1) % polygon.len()];
        if cross(a, b, p) == 0 && within(a, b, p) {
            return false;
        }
        if (a.y > p.y) != (b.y > p.y) {
            let (lhs, rhs) = ((p.x - a.x) * (b.y - a.y), (p.y - a.y) * (b.x - a.x));
            if if b.y > a.y { lhs < rhs } else { lhs > rhs } {
                odd = !odd;
            }
        }
    }
    odd
}

fn distance(a: Point, b: Point, p: Point) -> f64 {
    let (dx, dy) = ((b.x - a.x) as f64, (b.y - a.y) as f64);
    let (px, py) = ((p.x - a.x) as f64, (p.y - a.y) as f64);
    let length = dx * dx + dy * dy;
    let t = if length == 0.0 { 0.0 } else { ((px * dx + py * dy) / length).clamp(0.0, 1.0) };
    ((px - t * dx).powi(2) + (py - t * dy).powi(2)).sqrt()
}

/// One loop per link of a breadth-first spanning tree: the link from `a` to `b`, then the tree
/// path from `b` back to `a`. The anchor is the middle of the loop's parts.
fn tree_loops(edges: &[Edge], nodes: &[Node], links: &[Link]) -> Vec<(Vec<Step>, Anchor)> {
    let mut adjacent: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (l, link) in links.iter().enumerate() {
        adjacent[link.a].push(l);
        adjacent[link.b].push(l);
    }
    let mut parent: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut depth = vec![usize::MAX; nodes.len()];
    let mut tree = vec![false; links.len()];
    for root in 0..nodes.len() {
        if depth[root] != usize::MAX {
            continue;
        }
        depth[root] = 0;
        let mut queue = std::collections::VecDeque::from([root]);
        while let Some(n) = queue.pop_front() {
            for &l in &adjacent[n] {
                let other = if links[l].a == n { links[l].b } else { links[l].a };
                if depth[other] == usize::MAX {
                    depth[other] = depth[n] + 1;
                    parent[other] = Some(l);
                    tree[l] = true;
                    queue.push_back(other);
                }
            }
        }
    }
    let up = |n: usize| {
        let l = parent[n].expect("only the root has no parent");
        (l, if links[l].a == n { links[l].b } else { links[l].a })
    };
    let mut loops = Vec::new();
    for (l, link) in links.iter().enumerate().filter(|&(l, _)| !tree[l]) {
        let mut steps = vec![Step { link: l, forward: true }];
        let (mut from, mut to) = (link.b, link.a);
        let mut down = Vec::new();
        while from != to {
            if depth[from] >= depth[to] {
                let (p, above) = up(from);
                steps.push(Step { link: p, forward: links[p].a == from });
                from = above;
            } else {
                let (p, above) = up(to);
                down.push(Step { link: p, forward: links[p].b == to });
                to = above;
            }
        }
        steps.extend(down.into_iter().rev());
        let anchor = centroid(steps.iter().flat_map(|s| {
            let edge = &edges[links[s.link].edge];
            [edge.a, edge.b]
        }));
        loops.push((steps, anchor));
    }
    loops
}

/// Loops as vectors over the links, kept in echelon form to tell whether a new one is independent.
#[derive(Default)]
struct Basis {
    rows: Vec<(usize, Vec<RBig>)>,
}

impl Basis {
    fn insert(&mut self, steps: &[Step], links: usize) -> bool {
        let mut row = vec![RBig::ZERO; links];
        for s in steps {
            row[s.link] += if s.forward { RBig::ONE } else { -RBig::ONE };
        }
        for (pivot, basis) in &self.rows {
            if row[*pivot] != RBig::ZERO {
                let factor = row[*pivot].clone() / basis[*pivot].clone();
                for (r, b) in row.iter_mut().zip(basis) {
                    *r -= factor.clone() * b;
                }
            }
        }
        match row.iter().position(|r| *r != RBig::ZERO) {
            Some(pivot) => {
                self.rows.push((pivot, row));
                true
            }
            None => false,
        }
    }
}

