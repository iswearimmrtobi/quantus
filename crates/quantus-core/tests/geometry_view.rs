use quantus_core::geometry::view::{project, Bounds, Camera, Figure, Primitive, Shown, Stroke};
use quantus_core::geometry::{Line, Point, Vector3};

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
    let primitives = project(&[], &Camera::default(), &unit_box());
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
    let primitives = project(&[], &Camera::default(), &bounds);
    let labels: Vec<&str> =
        primitives.iter().filter_map(|p| if let Primitive::Label { text, .. } = p { Some(text.as_str()) } else { None }).collect();
    for tick in ["0.05", "0.1", "0.3", "10", "40", "2", "6"] {
        assert!(labels.contains(&tick), "{tick} in {labels:?}");
    }
}

#[test]
fn an_axis_seen_end_on_has_no_ticks() {
    let above = Camera::default().from_above();
    let primitives = project(&[], &above, &unit_box());
    let ticks = primitives.iter().filter(|p| matches!(p, Primitive::Segment { stroke: Stroke::Tick, .. })).count();
    assert_eq!(ticks, 8);
}

#[test]
fn points_get_a_marker_and_their_name() {
    let a = Figure::point(&Point::from_ints(1, 2, 3)).unwrap();
    assert_eq!(a, Figure::Point([1.0, 2.0, 3.0]));
    let camera = Camera { yaw: 0.0, pitch: 0.0, zoom: 1.0, centre: [0.0; 3] };
    let primitives = project(&[shown("A", a)], &camera, &unit_box());
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

fn segments(primitives: &[Primitive], of: usize) -> Vec<([f64; 2], [f64; 2])> {
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
    let primitives = project(&[shown("g", figure)], &camera, &unit_box());
    let lines = segments(&primitives, 0);
    assert_eq!(lines.len(), 1);
    assert!(close2(lines[0].0, [-2.0, 0.0]) && close2(lines[0].1, [2.0, 0.0]), "{lines:?}");
    assert!(primitives.iter().any(|p| matches!(p, Primitive::Label { text, at, .. } if text == "g" && close2(*at, [2.0, 0.0]))));

    let outside = Figure::Line { support: [0.0, 5.0, 0.0], direction: [1.0, 0.0, 0.0] };
    assert!(segments(&project(&[shown("h", outside)], &camera, &unit_box()), 0).is_empty());
}

#[test]
fn a_vector_is_an_arrow_from_the_origin() {
    let figure = Figure::vector(&Vector3::from_ints(1, 0, 0)).unwrap();
    let camera = Camera::default().from_above();
    let primitives = project(&[shown("a", figure)], &camera, &unit_box());
    let strokes = segments(&primitives, 0);
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
