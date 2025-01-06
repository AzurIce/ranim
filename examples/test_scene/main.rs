use std::time::Duration;

use env_logger::Env;
use glam::vec2;
use ranim::{
    animation::{creation::{Create, Uncreate}, fading::Fading, transform::Transform, write::{Unwrite, Write}},
    prelude::*,
    rabject::rabject2d::{
        bez_path::{BezPath, FillOptions, StrokeOptions},
        vmobject::{
            geometry::{Arc, Polygon, Square},
            svg::Svg,
            VMobject,
        },
    },
    scene::{canvas::Canvas, EntityId, Scene, SceneBuilder},
    typst_svg, typst_tree,
};
use vello::kurbo;

fn create_and_uncreate(scene: &mut Scene, canvas: &EntityId<Canvas>, vmobject: VMobject) {
    let vmobject = scene.play_in_canvas(&canvas, vmobject, Create::new() );
    scene.wait(Duration::from_secs_f32(0.5));
    scene.play_remove_in_canvas(&canvas, vmobject, Uncreate::new());
    scene.wait(Duration::from_secs_f32(0.5));
}

fn write_and_unwrite(scene: &mut Scene, canvas: &EntityId<Canvas>, vmobject: VMobject) {
    let vmobject = scene.play_in_canvas(&canvas, vmobject, Write::new());
    scene.wait(Duration::from_secs_f32(0.5));
    scene.play_remove_in_canvas(&canvas, vmobject, Unwrite::new());
    scene.wait(Duration::from_secs_f32(0.5));
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();

    let mut scene = SceneBuilder::new("test_scene").build();
    let canvas = scene.insert_new_canvas(1920, 1080);
    scene.center_canvas_in_frame(&canvas);
    let center = (1920.0 / 2.0, 1080.0 / 2.0);

    let typ = "#text(60pt)[Ranim]";

    let svg = typst_tree!(typ);
    let mut svg = Svg::from_tree(svg).build();
    println!("{:?}", svg.subpaths[0].stroke);
    svg.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - svg.bounding_box().center());

    write_and_unwrite(&mut scene, &canvas, svg.clone());

    // let mut square = Square::new(100.0).build();
    // square.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - square.bounding_box().center());

    // let _square = scene.play_in_canvas(&canvas, svg, Transform::new(square));

    let mut polygon = Polygon::new(vec![
        vec2(0.0, 0.0),
        vec2(-100.0, -200.0),
        vec2(400.0, 0.0),
        vec2(-100.0, 200.0),
    ])
    .with_stroke_width(10.0)
    .build();
    polygon.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - polygon.bounding_box().center());
    
    write_and_unwrite(&mut scene, &canvas, polygon.clone());

    let polygon = scene.play_in_canvas(&canvas, svg, Transform::new(polygon));

    let svg = typst_tree!(typ);
    let mut svg = Svg::from_tree(svg).build();
    svg.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - svg.bounding_box().center());

    let _svg = scene.play_in_canvas(&canvas, polygon, Transform::new(svg));

    // {
    //     let canvas = scene.get(&canvas);
    //     let data = get_texture_data(&scene.ctx.wgpu_ctx, &canvas.camera.render_texture);
    //     let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_raw(
    //         canvas.camera.viewport_width,
    //         canvas.camera.viewport_height,
    //         data,
    //     )
    //     .unwrap();
    //     image.save("test_scene_canvas.png").unwrap();
    // }

    // scene.render_to_image("Ghostscript_Tiger.png");

    // let mut polygon = Polygon::new(vec![
    //     vec2(-100.0, -300.0),
    //     vec2(500.0, 0.0),
    //     vec2(0.0, 700.0),
    //     vec2(200.0, 300.0),
    //     vec2(0.0, 0.0),
    // ])
    // .with_stroke_width(10.0)
    // .build();
    // polygon
    //     .rotate(
    //         std::f32::consts::PI / 4.0,
    //         Vec3::Z,
    //         TransformAnchor::origin(),
    //     )
    //     .set_color(palettes::manim::BLUE_C)
    //     .set_fill_color(palettes::manim::BLUE_C.with_alpha(0.5));

    // let mut arc = Arc::new(std::f32::consts::PI / 2.0)
    //     .with_radius(100.0)
    //     .with_stroke_width(20.0)
    //     .build();
    // arc.set_color(palettes::manim::RED_C);
    // arc.shift(vec3(-100.0, 100.0, 1.0));

    // let arc = scene.insert(arc);
    // let vgroup1 = scene.insert(VGroup::new(vec![arc, polygon]));
    // scene.play(&vgroup1, Fading::fade_in());

    // let mut circle = Circle::new(100.0).build();
    // circle.shift(vec3(-100.0, 0.0, 0.0));
    // let mut square = Square::new(100.0).build();
    // square.shift(vec3(100.0, 0.0, 0.0));
    // let vgroup2 = VGroup::new(vec![circle, square]);

    // scene.play(&vgroup1, Transform::new(vgroup2.clone()));
    // scene.remove(vgroup1);
    // let vgroup2 = scene.insert(vgroup2);

    // scene.wait(Duration::from_secs_f32(0.5));

    // let mut ellipse = Ellipse::new(100.0, 200.0).build();
    // ellipse
    //     .set_color(palettes::manim::YELLOW_B.with_alpha(0.5))
    //     .set_stroke_color(palettes::manim::YELLOW_B);

    // let mut dot = Dot::new(vec3(0.0, -100.0, 0.0)).build();
    // dot.set_color(palettes::manim::GREEN_C);

    // let vgroup3 = VGroup::new(vec![dot, ellipse]);
    // scene.play_remove(vgroup2, Transform::new(vgroup3.clone()));

    // let vgroup3 = scene.insert(vgroup3);
    // scene.play_remove(vgroup3, Fading::fade_out());

    // info!(
    //     "Rendered {} frames in {:?}",
    //     scene.frame_count,
    //     start.elapsed()
    // );
}
