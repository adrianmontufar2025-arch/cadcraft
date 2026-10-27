//! A crossing box takes a text when it catches a corner of the box around its letters, and a POINT
//! when it catches the figure it draws (its `PDMODE` circle at an absolute `PDSIZE`), as Civil 3D
//! does: the box need not reach a letter's stroke nor the point itself. A click still finds text
//! on its letters.

use cadcraft_engine::doc::geom::Vec3;
use cadcraft_engine::doc::{Common, Drawing, EntityKind, Handle, Point, Space};
use cadcraft_engine::geom::{Bounds2, Vec2};
use cadcraft_engine::render;
use cadcraft_engine::select::select_window;
use serde_json::json;

fn add(d: &mut Drawing, k: EntityKind) -> Handle {
    d.add(&Space::Model, Common::default(), k).unwrap()
}

fn crossing(d: &Drawing, a: Vec2, b: Vec2) -> Vec<Handle> {
    select_window(d, &Space::Model, Bounds2::new(a, b), true)
}

#[test]
fn a_crossing_box_takes_a_turned_text_by_an_empty_corner_of_its_letters() {
    let mut d = Drawing::new_metric();
    let angle = 30f64.to_radians();
    let k = EntityKind::Text(
        serde_json::from_value(json!({ "insert": { "x": 50.0, "y": 20.0, "z": 0.0 }, "height": 10.0, "value": "L", "rotation": angle })).unwrap(),
    );
    let EntityKind::Text(t) = &k else { unreachable!() };
    let strokes = render::place_text_entity(&d, t, &t.value).0.strokes;
    // The top right corner of the box around the "L", turned with the text: no stroke there.
    let local = Bounds2::from_points(strokes.iter().flatten().map(|q| q.rotate(-angle)));
    let corner = local.max.rotate(angle);
    let l = add(&mut d, k);
    let near = |c: Vec2| strokes.iter().flatten().any(|q| q.dist(c) < 1.5);
    assert!(!near(corner), "the corner is empty");
    // A small box over the corner takes it; the same box moved off the corner does not.
    let r = Vec2::new(0.4, 0.4);
    assert_eq!(crossing(&d, corner - r, corner + r), vec![l]);
    let off = corner + Vec2::new(1.0, 0.0).rotate(angle) * 2.0 + Vec2::new(0.0, 1.0).rotate(angle) * 2.0;
    assert!(crossing(&d, off - r, off + r).is_empty());
    // A window (not crossing) over the corner does not.
    assert!(select_window(&d, &Space::Model, Bounds2::new(corner - r, corner + r), false).is_empty());
}

#[test]
fn a_crossing_box_takes_a_point_by_the_circle_it_draws() {
    let mut d = Drawing::new_metric();
    d.header.set_i64("PDMODE", 32);
    d.header.set_f64("PDSIZE", 2.0);
    let p = add(&mut d, EntityKind::Point(Point { p: Vec3::new(100.0, 100.0, 0.0), angle: 0.0 }));
    // Over the top right of the circle (radius 1), away from the point itself.
    assert_eq!(crossing(&d, Vec2::new(100.6, 100.6), Vec2::new(101.2, 101.2)), vec![p]);
    // Outside the circle.
    assert!(crossing(&d, Vec2::new(101.0, 101.0), Vec2::new(101.5, 101.5)).is_empty());
    // A window must still hold it.
    assert!(select_window(&d, &Space::Model, Bounds2::new(Vec2::new(100.6, 100.6), Vec2::new(101.2, 101.2)), false).is_empty());
}
