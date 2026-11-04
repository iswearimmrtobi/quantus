use quantus_core::geometry::view::{project, Bounds, Camera, Figure, Options, Primitive, Shown, Stroke, RING};
use quantus_core::geometry::{Circle, Line, Plane, Point, Sphere, Vector3};
use quantus_core::num::Value;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn close2(a: [f64; 2], b: [f64; 2]) -> bool {
    close(a[0], b[0]) && close(a[1], b[1])
}

fn shown(name: &str, figure: Figure) -> Shown {
    Shown { name: name.to_owned(), figure }
}

fn unit_box() -> Bounds {
    Bounds { min: [-2.0; 3], max: [2.0; 3] }
}

#[test]
fn the_camera_projects_onto_the_screen_with_depth() {
    let front = Camera { yaw: 0.0, pitch: 0.0, zoom: 2.0, centre: [0.0; 3] };
    // From +x: y to the right, z up, x towards the viewer.
    let (at, depth) = front.project([3.0, 1.0, 2.0]);
    assert!(close2(at, [2.0, 4.0]));
    assert!(close(depth, 3.0));

    let above = Camera::default().from_above();
    let (at, depth) = above.project([1.0, 2.0, 5.0]);
    assert!(close2(at, [1.0, 2.0]), "{at:?}");
    assert!(close(depth, 5.0));

    let moved = Camera { centre: [1.0, 1.0, 1.0], ..front };
    let (at, _) = moved.project([1.0, 1.0, 1.0]);
    assert!(close2(at, [0.0, 0.0]));
}

#[test]
fn the_view_from_above_has_x_to_the_right_and_y_up() {
    let above = Camera::default().from_above();
    let (x, _) = above.project([1.0, 0.0, 0.0]);
    let (y, _) = above.project([0.0, 1.0, 0.0]);
    assert!(close2(x, [1.0, 0.0]), "x: {x:?}");
    assert!(close2(y, [0.0, 1.0]), "y: {y:?}");
}

#[test]
fn the_school_view_puts_x_to_the_lower_left() {
    let camera = Camera::default();
    let (x, _) = camera.project([1.0, 0.0, 0.0]);
    let (y, _) = camera.project([0.0, 1.0, 0.0]);
    let (z, _) = camera.project([0.0, 0.0, 1.0]);
    assert!(x[0] < 0.0 && x[1] < 0.0, "{x:?}");
    assert!(y[0] > 0.0, "{y:?}");
    assert!(close(z[0], 0.0) && z[1] > 0.0, "{z:?}");
}

#[test]
fn axes_have_ticks_numbers_and_names() {
    let primitives = project(&[], &Camera::default(), &unit_box(), &Options::default());
    let axes = primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Axis, .. })).count();
    assert_eq!(axes, 3);
    let labels: Vec<&str> =
        primitives.iter().filter_map(|p| if let Primitive::Label { text, .. } = p { Some(text.as_str()) } else { None }).collect();
    for name in ["x", "y", "z"] {
        assert!(labels.contains(&name), "{labels:?}");
    }
    // From −2 to 2 in steps of 1, without 0: four ticks per axis.
    let ticks = primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Tick, .. })).count();
    assert_eq!(ticks, 12);
    assert_eq!(labels.iter().filter(|l| **l == "-2").count(), 3);
    assert!(!labels.contains(&"0"));
}

#[test]
fn ticks_take_a_round_step() {
    let bounds = Bounds { min: [0.0; 3], max: [0.3, 40.0, 7.0] };
    let primitives = project(&[], &Camera::default(), &bounds, &Options::default());
    let labels: Vec<&str> =
        primitives.iter().filter_map(|p| if let Primitive::Label { text, .. } = p { Some(text.as_str()) } else { None }).collect();
    for tick in ["0.05", "0.1", "0.3", "10", "40", "2", "6"] {
        assert!(labels.contains(&tick), "{tick} in {labels:?}");
    }
}

#[test]
fn an_axis_seen_end_on_has_no_ticks() {
    let above = Camera::default().from_above();
    let primitives = project(&[], &above, &unit_box(), &Options::default());
    let ticks = primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Tick, .. })).count();
    assert_eq!(ticks, 8);
}

#[test]
fn points_get_a_marker_and_their_name() {
    let a = Figure::point(&Point::from_ints(1, 2, 3)).unwrap();
    assert_eq!(a, Figure::Point([1.0, 2.0, 3.0]));
    let camera = Camera { yaw: 0.0, pitch: 0.0, zoom: 1.0, centre: [0.0; 3] };
    let primitives = project(&[shown("A", a)], &camera, &unit_box(), &Options::default());
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Marker { at, object: 0, .. } if close2(*at, [2.0, 3.0]))));
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Label { text, object: Some(0), .. } if text == "A")));
}

#[test]
fn the_box_takes_the_scene_and_the_origin_with_a_margin() {
    let figures = [shown("A", Figure::Point([1.0, 2.0, 3.0])), shown("B", Figure::Point([5.0, -2.0, 3.0]))];
    let bounds = Bounds::fit(&figures);
    // x from 0 to 5, y from −2 to 2, z from 0 to 3, each widened by a tenth.
    assert!(close(bounds.min[0], -0.25) && close(bounds.max[0], 5.25), "{bounds:?}");
    assert!(close(bounds.min[1], -2.2) && close(bounds.max[1], 2.2), "{bounds:?}");
    assert!(close(bounds.min[2], -0.15) && close(bounds.max[2], 3.15), "{bounds:?}");
}

#[test]
fn an_empty_scene_gets_a_box_around_the_origin() {
    let bounds = Bounds::fit(&[]);
    assert_eq!(bounds, Bounds { min: [-1.0; 3], max: [1.0; 3] });
}

#[test]
fn the_fitted_camera_keeps_the_box_in_the_unit_circle() {
    let figures = [shown("A", Figure::Point([4.0, -3.0, 6.0])), shown("B", Figure::Point([-1.0, 2.0, 0.5]))];
    let bounds = Bounds::fit(&figures);
    for (yaw, pitch) in [(0.3, 0.2), (2.0, -1.0), (-1.2, 1.5)] {
        let camera = Camera { yaw, pitch, ..Camera::default() }.fit(&bounds);
        assert!(close(camera.yaw, yaw) && close(camera.pitch, pitch));
        for corner in 0..8 {
            let p = [0, 1, 2].map(|i| if corner >> i & 1 == 1 { bounds.max[i] } else { bounds.min[i] });
            let (at, _) = camera.project(p);
            assert!(at[0].hypot(at[1]) <= 1.0 + 1e-9, "{at:?}");
        }
        let centre = [0, 1, 2].map(|i| (bounds.min[i] + bounds.max[i]) / 2.0);
        assert!(close2(camera.project(centre).0, [0.0, 0.0]));
    }
}

fn object_segments(primitives: &[Primitive], of: usize) -> Vec<([f64; 2], [f64; 2])> {
    primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Segment { from, to, object: Some(o), .. } if *o == of => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_line_is_clipped_to_the_box() {
    let bounds = unit_box();
    let (a, b) = bounds.clip([0.0, 0.0, 0.0], [1.0, 1.0, 0.0]).unwrap();
    assert_eq!((a, b), ([-2.0, -2.0, 0.0], [2.0, 2.0, 0.0]));
    let (a, b) = bounds.clip([0.0, 1.0, 5.0], [0.0, 0.0, -2.0]).unwrap();
    assert_eq!((a, b), ([0.0, 1.0, 2.0], [0.0, 1.0, -2.0]));
    // Parallel to a face outside the box, and past a corner.
    assert_eq!(bounds.clip([0.0, 3.0, 0.0], [1.0, 0.0, 0.0]), None);
    assert_eq!(bounds.clip([5.0, 0.0, 0.0], [1.0, 1.0, 0.0]), None);
}

#[test]
fn a_line_is_one_segment_with_its_name_at_the_end() {
    let g = Line::new(Point::from_ints(0, 0, 1), Vector3::from_ints(1, 0, 0)).unwrap();
    let figure = Figure::line(&g).unwrap();
    assert_eq!(figure, Figure::Line { support: [0.0, 0.0, 1.0], direction: [1.0, 0.0, 0.0] });
    let camera = Camera::default().from_above();
    let primitives = project(&[shown("g", figure)], &camera, &unit_box(), &Options::default());
    let lines = object_segments(&primitives, 0);
    assert_eq!(lines.len(), 1);
    assert!(close2(lines[0].0, [-2.0, 0.0]) && close2(lines[0].1, [2.0, 0.0]), "{lines:?}");
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Label { text, at, .. } if text == "g" && close2(*at, [2.0, 0.0]))));

    let outside = Figure::Line { support: [0.0, 5.0, 0.0], direction: [1.0, 0.0, 0.0] };
    assert!(object_segments(&project(&[shown("h", outside)], &camera, &unit_box(), &Options::default()), 0).is_empty());
}

#[test]
fn a_vector_is_an_arrow_from_the_origin() {
    let figure = Figure::vector(&Vector3::from_ints(1, 0, 0)).unwrap();
    let camera = Camera::default().from_above();
    let primitives = project(&[shown("a", figure)], &camera, &unit_box(), &Options::default());
    let strokes = object_segments(&primitives, 0);
    assert_eq!(strokes.len(), 3);
    assert!(close2(strokes[0].0, [0.0, 0.0]) && close2(strokes[0].1, [1.0, 0.0]));
    // The head points back from the tip, one stroke to each side.
    for (from, to) in &strokes[1..] {
        assert!(close2(*from, [1.0, 0.0]) && to[0] < 1.0);
    }
    assert!(strokes[1].1[1] * strokes[2].1[1] < 0.0);
}

#[test]
fn the_box_takes_the_support_of_a_line_and_the_tip_of_a_vector() {
    let figures = [
        shown("g", Figure::Line { support: [10.0, 0.0, 0.0], direction: [0.0, 1.0, 0.0] }),
        shown("a", Figure::Vector([0.0, 0.0, -10.0])),
    ];
    let bounds = Bounds::fit(&figures);
    assert!(close(bounds.max[0], 10.5) && close(bounds.min[2], -10.5), "{bounds:?}");
}

fn polygons(primitives: &[Primitive]) -> Vec<(Vec<[f64; 2]>, usize)> {
    primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Polygon { points, object, .. } => Some((points.clone(), *object)),
            _ => None,
        })
        .collect()
}

fn hatches(primitives: &[Primitive]) -> usize {
    primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Hatch, .. })).count()
}

#[test]
fn a_plane_cuts_the_box_in_a_polygon() {
    let bounds = unit_box();
    // z = 1: a square.
    let square = bounds.cut([0.0, 0.0, 1.0], 1.0);
    assert_eq!(square.len(), 4);
    assert!(square.iter().all(|p| close(p[2], 1.0) && close(p[0].abs(), 2.0) && close(p[1].abs(), 2.0)));
    // x + y + z = 0: a regular hexagon through the middles of six edges.
    let hexagon = bounds.cut([1.0, 1.0, 1.0], 0.0);
    assert_eq!(hexagon.len(), 6);
    for i in 0..6 {
        let (p, q) = (hexagon[i], hexagon[(i + 1) % 6]);
        let side = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
        assert!(close(side, 2.0 * 2f64.sqrt()), "{hexagon:?}");
    }
    // x + y + z = 5: a triangle at the corner (2|2|2).
    assert_eq!(bounds.cut([1.0, 1.0, 1.0], 5.0).len(), 3);
    // Through a face, along an edge, or past the box.
    assert_eq!(bounds.cut([1.0, 0.0, 0.0], 2.0).len(), 4);
    assert!(bounds.cut([1.0, 1.0, 0.0], 4.0).is_empty());
    assert!(bounds.cut([0.0, 0.0, 1.0], 3.0).is_empty());
}

#[test]
fn a_plane_is_a_polygon_with_its_name_and_an_optional_hatch() {
    let e = Plane::coordinate(Value::zero(), Value::zero(), Value::one(), Value::one()).unwrap();
    let figure = Figure::plane(&e).unwrap();
    assert_eq!(figure, Figure::Plane { normal: [0.0, 0.0, 1.0], constant: 1.0, support: [0.0, 0.0, 1.0] });
    let camera = Camera::default().from_above();
    let primitives = project(&[shown("E", figure.clone())], &camera, &unit_box(), &Options::default());
    let pieces = polygons(&primitives);
    assert_eq!(pieces.len(), 1);
    assert_eq!(pieces[0].0.len(), 4);
    assert_eq!(hatches(&primitives), 0);
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Label { text, at, .. } if text == "E" && close2(*at, [2.0, 2.0]))));

    let hatched = project(&[shown("E", figure)], &camera, &unit_box(), &Options { hatch: true });
    assert!(hatches(&hatched) > 5, "{}", hatches(&hatched));
    // Every hatch line runs inside the square.
    for p in &hatched {
        if let Primitive::Segment { from, to, stroke: Stroke::Hatch, .. } = p {
            for end in [from, to] {
                assert!(end[0].abs() <= 2.0 + 1e-9 && end[1].abs() <= 2.0 + 1e-9, "{end:?}");
            }
        }
    }
}

#[test]
fn a_plane_past_the_box_draws_nothing() {
    let figure = Figure::Plane { normal: [0.0, 0.0, 1.0], constant: 9.0, support: [0.0, 0.0, 9.0] };
    let primitives = project(&[shown("E", figure)], &Camera::default(), &unit_box(), &Options { hatch: true });
    assert!(polygons(&primitives).is_empty());
    assert!(!primitives.iter().any(|p| matches!(p, Primitive::Label { object: Some(0), .. })));
}

fn strokes(primitives: &[Primitive], stroke: Stroke) -> Vec<([f64; 2], [f64; 2])> {
    primitives
        .iter()
        .filter_map(|p| match p {
            Primitive::Segment { from, to, stroke: s, object: Some(_), .. } if *s == stroke => Some((*from, *to)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_sphere_is_an_outline_with_rings_their_far_halves_hidden() {
    let k = Sphere::new(&Point::from_ints(1, 0, 0), Value::int(2)).unwrap();
    let figure = Figure::sphere(&k).unwrap();
    assert_eq!(figure, Figure::Sphere { centre: [1.0, 0.0, 0.0], radius: 2.0 });
    let camera = Camera { zoom: 0.5, ..Camera::default() };
    let primitives = project(&[shown("K", figure)], &camera, &unit_box(), &Options::default());
    let outline = polygons(&primitives);
    assert_eq!(outline.len(), 1);
    let (middle, _) = camera.project([1.0, 0.0, 0.0]);
    for p in &outline[0].0 {
        assert!(close((p[0] - middle[0]).hypot(p[1] - middle[1]), 1.0), "{p:?}");
    }
    // Three latitudes and four meridians, each a ring of short segments inside the outline.
    let front = strokes(&primitives, Stroke::Object);
    let back = strokes(&primitives, Stroke::Hidden);
    assert_eq!(front.len() + back.len(), 7 * RING);
    assert!(!front.is_empty() && !back.is_empty());
    for (from, to) in front.iter().chain(&back) {
        for end in [from, to] {
            assert!((end[0] - middle[0]).hypot(end[1] - middle[1]) <= 1.0 + 1e-9);
        }
    }
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Label { text, .. } if text == "K")));
}

#[test]
fn a_circle_is_a_ring_of_segments_in_its_plane() {
    let k = Circle::new(&Point::from_ints(0, 0, 1), Value::int(2), &Vector3::from_ints(0, 0, 3)).unwrap();
    let figure = Figure::circle(&k).unwrap();
    let camera = Camera::default().from_above();
    let primitives = project(&[shown("k", figure)], &camera, &unit_box(), &Options::default());
    let ring = strokes(&primitives, Stroke::Object);
    assert_eq!(ring.len(), RING);
    assert!(strokes(&primitives, Stroke::Hidden).is_empty());
    // From above, a circle in a horizontal plane is a circle of radius 2 on the screen, closed.
    for (from, _) in &ring {
        assert!(close(from[0].hypot(from[1]), 2.0), "{from:?}");
    }
    // Closed: every segment ends where another one starts.
    assert!(ring.iter().all(|(_, to)| ring.iter().any(|(from, _)| close2(*from, *to))));
}

#[test]
fn the_box_takes_spheres_and_circles_whole() {
    let figures = [
        shown("K", Figure::Sphere { centre: [0.0, 0.0, 0.0], radius: 10.0 }),
        shown("k", Figure::Circle { centre: [0.0, 0.0, 20.0], radius: 10.0, normal: [0.0, 0.0, 1.0] }),
    ];
    let bounds = Bounds::fit(&figures);
    // x and y from −10 to 10, z from −10 to 20 (the circle lies flat there), with a tenth more.
    assert!(close(bounds.max[0], 11.0) && close(bounds.min[1], -11.0), "{bounds:?}");
    assert!(close(bounds.min[2], -11.5) && close(bounds.max[2], 21.5), "{bounds:?}");
}

fn plane(normal: [f64; 3], constant: f64) -> Figure {
    Figure::Plane { normal, constant, support: [0.0; 3] }
}

#[test]
fn primitives_come_far_to_near_with_the_labels_last() {
    let figures = [
        shown("A", Figure::Point([1.0, 1.0, 1.0])),
        shown("g", Figure::Line { support: [0.0; 3], direction: [1.0, 2.0, 0.5] }),
        shown("E", plane([0.0, 1.0, 1.0], 0.5)),
        shown("K", Figure::Sphere { centre: [0.0; 3], radius: 1.0 }),
    ];
    let primitives = project(&figures, &Camera::default(), &unit_box(), &Options { hatch: true });
    let first_label = primitives.iter().position(|p| matches!(p, Primitive::Label { .. })).unwrap();
    assert!(primitives[first_label..].iter().all(|p| matches!(p, Primitive::Label { .. })));
    for part in [&primitives[..first_label], &primitives[first_label..]] {
        assert!(part.windows(2).all(|w| w[0].depth() <= w[1].depth()));
    }
}

#[test]
fn crossing_planes_are_split_along_their_intersection_line() {
    let figures = [shown("E", plane([1.0, 0.0, 0.0], 0.0)), shown("F", plane([0.0, 2.0, 0.0], 0.0))];
    let primitives = project(&figures, &Camera::default(), &unit_box(), &Options::default());
    let pieces = polygons(&primitives);
    assert_eq!(pieces.len(), 4);
    assert_eq!(pieces.iter().filter(|(_, o)| *o == 0).count(), 2);
    assert!(pieces.iter().all(|(points, _)| points.len() == 4));
    // Each piece is half of its square, two corners on the intersection line, the z axis.
    let camera = Camera::default();
    let (top, _) = camera.project([0.0, 0.0, 2.0]);
    let (bottom, _) = camera.project([0.0, 0.0, -2.0]);
    for (points, _) in &pieces {
        assert!(points.iter().any(|p| close2(*p, top)) && points.iter().any(|p| close2(*p, bottom)), "{points:?}");
    }
}

#[test]
fn parallel_and_identical_planes_are_not_split() {
    let figures = [shown("E", plane([0.0, 0.0, 1.0], 0.0)), shown("F", plane([0.0, 0.0, 1.0], 1.0)), shown("G", plane([0.0, 0.0, 2.0], 0.0))];
    let primitives = project(&figures, &Camera::default(), &unit_box(), &Options::default());
    assert_eq!(polygons(&primitives).len(), 3);
}

#[test]
fn a_line_and_the_axes_are_cut_where_they_pass_through_a_plane() {
    let figures = [shown("g", Figure::Line { support: [0.0, 0.5, 0.5], direction: [1.0, 0.0, 0.0] }), shown("E", plane([1.0, 0.0, 0.0], 1.0))];
    let primitives = project(&figures, &Camera::default(), &unit_box(), &Options::default());
    assert_eq!(object_segments(&primitives, 0).len(), 2);
    // The x axis passes through x = 1 too; y and z lie beside it.
    let axes = primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Axis, .. })).count();
    assert_eq!(axes, 4);
}
