//! Junctions, branches and their currents: the unknowns of the branch-current method.
//!
//! A junction is an electrical node with three or more connections. A branch is a chain of parts
//! between two junctions, with one current through all of them; a connected part without a
//! junction is a single branch that closes on itself. Each branch's reference direction comes
//! from its lead part, the first by kind (voltage source, current source, resistor), then by ID.
//!
//! A source's internal resistance is part of the circuit too: a voltage source's lies in series in
//! its branch, a current source's (`1/G`) parallel to it, as a branch and a loop of its own.

use std::cmp::Ordering;

use crate::num::Value;

use super::graph::{Anchor, Edge, Graph};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    VoltageSource,
    CurrentSource,
    Resistor,
    Wire,
}

/// A part of the drawing, in the order of the edges the graph was built from.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub id: String,
    pub kind: Kind,
    /// R in Ω, U in V or I in A, exact; unused for a wire. A source's `+` terminal is `a`; a
    /// current source drives its current through itself from `a` to `b`.
    pub value: Value,
    /// A source's internal resistance in Ω, for a current source `1/G`, if it has one.
    pub internal: Option<Value>,
    /// Flips the reference direction of the branch this part leads.
    pub reversed: bool,
}

/// A part in a branch, or a source's internal resistance (`internal`). `forward` if the branch
/// current runs through it from terminal `a` to `b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub part: usize,
    pub internal: bool,
    pub forward: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    /// In the direction of the current.
    pub members: Vec<Member>,
    /// Index into `members` of the part that sets the direction.
    pub lead: usize,
    /// The nodes the current leaves and enters; the same node for a branch that closes on itself.
    pub from: usize,
    pub to: usize,
    pub part: usize,
}

/// One member of a branch on a loop, `along` if the loop runs with the branch current there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turn {
    pub branch: usize,
    pub member: usize,
    pub along: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    pub turns: Vec<Turn>,
    pub anchor: Anchor,
    pub part: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    /// Graph nodes, in reading order.
    pub junctions: Vec<usize>,
    /// In the order of their lead parts; branch `k` carries `I_{k+1}`.
    pub branches: Vec<Branch>,
    /// The graph's loops, plus one per current source with an internal conductance, in reading
    /// order of their anchors.
    pub loops: Vec<Mesh>,
}

/// A link of the graph, or the internal conductance of a current source beside it.
struct Connection {
    a: usize,
    b: usize,
    part: usize,
    internal: bool,
}

impl Connection {
    /// What the branch current passes: a voltage source's internal resistance goes with it.
    fn pieces(&self, parts: &[Part]) -> Vec<(usize, bool)> {
        let part = &parts[self.part];
        match part.kind {
            Kind::VoltageSource if !self.internal && part.internal.is_some() => vec![(self.part, false), (self.part, true)],
            _ => vec![(self.part, self.internal)],
        }
    }
}

impl Network {
    /// `parts` and `edges` are in the order the graph was built from.
    pub fn build(graph: &Graph, parts: &[Part], edges: &[Edge]) -> Network {
        let mut connections: Vec<Connection> =
            graph.links.iter().map(|l| Connection { a: l.a, b: l.b, part: l.edge, internal: false }).collect();
        let parallel: Vec<usize> = (0..graph.links.len())
            .filter(|&l| {
                let part = &parts[graph.links[l].edge];
                part.kind == Kind::CurrentSource && part.internal.is_some()
            })
            .collect();
        for &l in &parallel {
            let link = &graph.links[l];
            connections.push(Connection { a: link.a, b: link.b, part: link.edge, internal: true });
        }

        let mut adjacent: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
        for (c, connection) in connections.iter().enumerate() {
            adjacent[connection.a].push(c);
            adjacent[connection.b].push(c);
        }
        let junction: Vec<bool> = adjacent.iter().map(|a| a.len() >= 3).collect();
        let junctions: Vec<usize> = (0..graph.nodes.len()).filter(|&n| junction[n]).collect();

        // Chains of connections from junction to junction, then the cycles without one.
        let mut used = vec![false; connections.len()];
        let mut chains: Vec<Chain> = Vec::new();
        let starts = junctions.iter().flat_map(|&n| adjacent[n].iter().map(move |&c| (n, c)));
        let rest = (0..connections.len()).map(|c| (connections[c].a, c));
        for (start, first) in starts.collect::<Vec<_>>().into_iter().chain(rest) {
            if used[first] {
                continue;
            }
            let (mut node, mut c) = (start, first);
            let mut chain = Vec::new();
            loop {
                used[c] = true;
                let forward = connections[c].a == node;
                chain.push((c, forward));
                node = if forward { connections[c].b } else { connections[c].a };
                if junction[node] || node == start {
                    break;
                }
                match adjacent[node].iter().find(|&&next| !used[next]) {
                    Some(&next) => c = next,
                    None => break,
                }
            }
            chains.push((start, chain, node));
        }

        // Each branch with the connection of each of its members.
        let mut branches: Vec<(Branch, Vec<usize>)> = chains
            .into_iter()
            .map(|(from, chain, to)| {
                let (members, mut sources): (Vec<Member>, Vec<usize>) = chain
                    .iter()
                    .flat_map(|&(c, forward)| {
                        connections[c].pieces(parts).into_iter().map(move |(part, internal)| (Member { part, internal, forward }, c))
                    })
                    .unzip();
                let lead = (0..members.len()).min_by(|&m, &n| lead_order(parts, &members[m], &members[n])).unwrap_or(0);
                let mut branch = Branch { members, lead, from, to, part: graph.nodes[from].part };
                let lead = branch.members[branch.lead];
                if lead.forward != leads_forward(&parts[lead.part], lead.internal) {
                    branch.reverse();
                    sources.reverse();
                }
                (branch, sources)
            })
            .collect();
        branches.sort_by(|(p, _), (q, _)| lead_order(parts, &p.members[p.lead], &q.members[q.lead]));

        let mut place: Vec<Vec<(usize, usize)>> = vec![Vec::new(); connections.len()];
        for (b, (_, sources)) in branches.iter().enumerate() {
            for (m, &c) in sources.iter().enumerate() {
                place[c].push((b, m));
            }
        }
        let branches: Vec<Branch> = branches.into_iter().map(|(branch, _)| branch).collect();
        // A connection's members in the order the loop passes them.
        let turns = |c: usize, forward: bool| -> Vec<Turn> {
            let mut turns: Vec<Turn> = place[c]
                .iter()
                .map(|&(branch, member)| Turn { branch, member, along: forward == branches[branch].members[member].forward })
                .collect();
            if turns.first().is_some_and(|t| !t.along) {
                turns.reverse();
            }
            turns
        };

        let mut loops: Vec<Mesh> = graph
            .loops
            .iter()
            .map(|l| Mesh { turns: l.steps.iter().flat_map(|s| turns(s.link, s.forward)).collect(), anchor: l.anchor, part: l.part })
            .collect();
        // Through the source from `a` to `b`, back through its internal conductance.
        for (n, &l) in parallel.iter().enumerate() {
            let link = &graph.links[l];
            let edge = &edges[link.edge];
            let mut steps = turns(l, true);
            steps.extend(turns(graph.links.len() + n, false));
            let anchor = Anchor { x: (edge.a.x + edge.b.x) as f64 / 2.0, y: (edge.a.y + edge.b.y) as f64 / 2.0 };
            loops.push(Mesh { turns: steps, anchor, part: graph.nodes[link.a].part });
        }
        loops.sort_by(|p, q| (p.anchor.y, p.anchor.x).partial_cmp(&(q.anchor.y, q.anchor.x)).unwrap_or(Ordering::Equal));
        Network { junctions, branches, loops }
    }
}

/// A walk from a node to a node over connections, each `true` if walked from `a` to `b`.
type Chain = (usize, Vec<(usize, bool)>, usize);

impl Branch {
    fn reverse(&mut self) {
        self.members.reverse();
        for member in &mut self.members {
            member.forward = !member.forward;
        }
        self.lead = self.members.len() - 1 - self.lead;
        std::mem::swap(&mut self.from, &mut self.to);
    }
}

/// Whether the reference current runs through the lead from `a` to `b`: out of a voltage
/// source's `+` terminal, so from `b` to `a` inside it, the way a current source drives, and
/// from `a` to `b` through a resistor.
fn leads_forward(part: &Part, internal: bool) -> bool {
    let forward = internal || part.kind != Kind::VoltageSource;
    forward != part.reversed
}

/// Voltage sources, current sources, then resistors and internal resistances; by ID within a kind.
fn lead_order(parts: &[Part], m: &Member, n: &Member) -> Ordering {
    let rank = |member: &Member| match parts[member.part].kind {
        _ if member.internal => 2,
        Kind::VoltageSource => 0,
        Kind::CurrentSource => 1,
        Kind::Resistor | Kind::Wire => 2,
    };
    rank(m)
        .cmp(&rank(n))
        .then_with(|| id_order(&parts[m.part].id, &parts[n.part].id))
        .then_with(|| m.internal.cmp(&n.internal))
}

/// `R2` before `R10`: the letters, then the number after them by value.
pub(crate) fn id_order(p: &str, q: &str) -> Ordering {
    let split = |id: &str| {
        let digits = id.len() - id.chars().rev().take_while(|c| c.is_ascii_digit()).count();
        let (letters, number) = id.split_at(digits);
        (letters.to_string(), number.parse::<u64>().ok(), number.to_string())
    };
    split(p).cmp(&split(q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kirchhoff::graph::Point;

    fn edge(id: &str, a: (i64, i64), b: (i64, i64)) -> Edge {
        Edge { id: id.into(), a: Point::new(a.0, a.1), b: Point::new(b.0, b.1), wire: id.starts_with('W') }
    }

    fn part(id: &str) -> Part {
        let kind = match &id[..1] {
            "V" => Kind::VoltageSource,
            "I" => Kind::CurrentSource,
            "W" => Kind::Wire,
            _ => Kind::Resistor,
        };
        Part { id: id.into(), kind, value: Value::one(), internal: None, reversed: false }
    }

    fn network(edges: &[Edge], parts: &[Part]) -> Network {
        Network::build(&Graph::build(edges).unwrap(), parts, edges)
    }

    fn name(parts: &[Part], member: &Member, sign: bool) -> String {
        let id = &parts[member.part].id;
        format!("{}{}{id}", if sign { '+' } else { '-' }, if member.internal { "i" } else { "" })
    }

    /// Each branch as its members in the direction of the current, `+` where that runs from `a`
    /// to `b`.
    fn branches(parts: &[Part], network: &Network) -> Vec<String> {
        let members = |b: &Branch| b.members.iter().map(|m| name(parts, m, m.forward)).collect::<Vec<_>>().join(" ");
        network.branches.iter().map(members).collect()
    }

    /// Each loop as its members, `+` where it runs with the branch current.
    fn loops(parts: &[Part], network: &Network) -> Vec<String> {
        let turns = |l: &Mesh| {
            l.turns.iter().map(|t| name(parts, &network.branches[t.branch].members[t.member], t.along)).collect::<Vec<_>>().join(" ")
        };
        network.loops.iter().map(turns).collect()
    }

    fn single() -> Vec<Edge> {
        vec![edge("V1", (0, 4), (0, 0)), edge("R1", (0, 0), (4, 0)), edge("W1", (4, 0), (4, 4)), edge("W2", (4, 4), (0, 4))]
    }

    fn parts(edges: &[Edge]) -> Vec<Part> {
        edges.iter().map(|e| part(&e.id)).collect()
    }

    #[test]
    fn a_single_loop_is_one_branch_out_of_the_plus_terminal() {
        let edges = single();
        let mut parts = parts(&edges);
        let net = network(&edges, &parts);
        assert!(net.junctions.is_empty());
        assert_eq!(branches(&parts, &net), ["-R1 -V1"]);
        assert_eq!(net.branches[0].lead, 1);
        assert_eq!(net.branches[0].from, net.branches[0].to);
        // The loop runs clockwise, against the current.
        assert_eq!(loops(&parts, &net), ["-V1 -R1"]);

        parts[0].reversed = true;
        let net = network(&edges, &parts);
        assert_eq!(branches(&parts, &net), ["+V1 +R1"]);
        assert_eq!(loops(&parts, &net), ["+V1 +R1"]);
    }

    #[test]
    fn a_ladder_has_three_branches_between_two_junctions() {
        let edges = vec![
            edge("V1", (0, 4), (0, 0)),
            edge("R1", (0, 0), (4, 0)),
            edge("R2", (4, 0), (4, 4)),
            edge("R3", (4, 0), (8, 0)),
            edge("R4", (8, 0), (8, 4)),
            edge("W1", (0, 4), (4, 4)),
            edge("W2", (4, 4), (8, 4)),
        ];
        let parts = parts(&edges);
        let net = network(&edges, &parts);
        assert_eq!(net.junctions, [1, 3]);
        assert_eq!(branches(&parts, &net), ["-R1 -V1", "+R2", "+R3 +R4"]);
        assert_eq!((net.branches[0].from, net.branches[0].to), (1, 3));
        assert_eq!((net.branches[2].from, net.branches[2].to), (1, 3));
        assert_eq!(loops(&parts, &net), ["-V1 -R1 +R2", "-R2 +R3 +R4"]);
    }

    #[test]
    fn a_wheatstone_bridge_has_six_branches() {
        let edges = vec![
            edge("R1", (0, 0), (4, 0)),
            edge("R2", (4, 0), (8, 0)),
            edge("R3", (0, 8), (4, 8)),
            edge("R4", (4, 8), (8, 8)),
            edge("R5", (4, 2), (4, 6)),
            edge("V1", (2, 12), (6, 12)),
            edge("W1", (4, 0), (4, 2)),
            edge("W2", (4, 6), (4, 8)),
            edge("W3", (0, 0), (0, 8)),
            edge("W4", (0, 8), (0, 12)),
            edge("W5", (0, 12), (2, 12)),
            edge("W6", (6, 12), (8, 12)),
            edge("W7", (8, 12), (8, 8)),
            edge("W8", (8, 8), (8, 0)),
        ];
        let parts = parts(&edges);
        let net = network(&edges, &parts);
        assert_eq!(net.junctions.len(), 4);
        assert_eq!(branches(&parts, &net), ["-V1", "+R1", "+R2", "+R3", "+R4", "+R5"]);
        assert_eq!(loops(&parts, &net), ["+R1 +R5 -R3", "+R2 -R4 -R5", "+R3 +R4 +V1"]);
    }

    #[test]
    fn a_voltage_sources_internal_resistance_is_in_series() {
        let edges = single();
        let mut parts = parts(&edges);
        parts[0].internal = Some(Value::int(2));
        let net = network(&edges, &parts);
        assert_eq!(branches(&parts, &net), ["-R1 -iV1 -V1"]);
        assert_eq!(net.branches[0].lead, 2);
        assert_eq!(loops(&parts, &net), ["-V1 -iV1 -R1"]);
    }

    #[test]
    fn a_current_sources_internal_conductance_is_a_branch_and_a_loop() {
        let mut edges = single();
        edges[0].id = "I1".into();
        let mut parts = parts(&edges);
        parts[0].internal = Some(Value::int(2));
        let net = network(&edges, &parts);
        assert_eq!(net.junctions.len(), 2);
        assert_eq!(branches(&parts, &net), ["+I1", "+iI1", "+R1"]);
        assert_eq!(loops(&parts, &net), ["+I1 -iI1", "+I1 +R1"]);
    }

    #[test]
    fn ids_sort_by_number() {
        assert_eq!(id_order("R2", "R10"), Ordering::Less);
        assert_eq!(id_order("R10", "V1"), Ordering::Less);
    }
}
