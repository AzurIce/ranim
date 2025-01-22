use std::time::Duration;

use env_logger::Env;
use glam::{vec2, vec3};
use ranim::{
    animation::{creation, transform::Transform},
    items::vitem::{Square, VItem},
    prelude::*,
    rabject::rabject3d::RabjectEntity3d,
    render::Renderer,
    typst_svg, typst_tree,
    world::{EntityId, World},
    AppOptions, RanimApp, RanimRenderApp, Scenee,
};

// fn create_and_uncreate<T: RanimApp>(scene: &mut T, canvas: &EntityId<Canvas>, vmobject: VMobject) {
//     let vmobject = scene.play_in_canvas(&canvas, vmobject, creation::create());
//     scene.wait(Duration::from_secs_f32(0.5));
//     scene.play_remove_in_canvas(&canvas, vmobject, creation::uncreate());
//     scene.wait(Duration::from_secs_f32(0.5));
// }

// fn write_and_unwrite<T: RanimApp>(scene: &mut T, canvas: &EntityId<Canvas>, vmobject: VMobject) {
//     let vmobject = scene.play_in_canvas(&canvas, vmobject, creation::write());
//     scene.wait(Duration::from_secs_f32(0.5));
//     scene.play_remove_in_canvas(&canvas, vmobject, creation::unwrite());
//     scene.wait(Duration::from_secs_f32(0.5));
// }

// struct TestCanvasScene;

// impl Default for TestCanvasScene {
//     fn default() -> Self {
//         Self {}
//     }
// }

// impl Scenee for TestCanvasScene {
//     fn desc() -> ranim::SceneDesc {
//         ranim::SceneDesc {
//             name: "Test Canvas Scene".to_string(),
//         }
//     }
//     fn construct<T: ranim::RanimApp>(&mut self, app: &mut T) {
//         let canvas = app.insert_new_canvas(1920, 1080);
//         app.center_canvas_in_frame(&canvas);
//         let center = (1920.0 / 2.0, 1080.0 / 2.0);

//         let typ = "#text(60pt)[Ranim]";

//         let svg = typst_tree!(typ);
//         let mut svg = Svg::from_tree(svg).build();
//         println!("{:?}", svg.subpaths[0].stroke);
//         svg.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - svg.bounding_box().center());

//         write_and_unwrite(app, &canvas, svg.clone());

//         let mut polygon = Polygon::new(vec![
//             vec2(0.0, 0.0),
//             vec2(-100.0, -200.0),
//             vec2(400.0, 0.0),
//             vec2(-100.0, 200.0),
//         ])
//         .with_stroke_width(10.0)
//         .build();
//         polygon.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - polygon.bounding_box().center());

//         write_and_unwrite(app, &canvas, polygon.clone());

//         let polygon = app.play_in_canvas(&canvas, svg, Transform::new(polygon));

//         let svg = typst_tree!(typ);
//         let mut svg = Svg::from_tree(svg).build();
//         svg.shift(vec2(1920.0 / 2.0, 1080.0 / 2.0) - svg.bounding_box().center());

//         let _svg = app.play_in_canvas(&canvas, polygon, Transform::new(svg));
//     }
// }

#[derive(Default)]
struct TestScene;

impl Scenee for TestScene {
    fn desc() -> ranim::SceneDesc {
        ranim::SceneDesc {
            name: "Test Scene".to_string(),
        }
    }
    fn construct<T: ranim::RanimApp>(&mut self, app: &mut T) {
        let square: RabjectEntity3d<VItem> = Square(100.0).build().into();
        app.insert(square);
        // app.render_to_image("test.png");
        app.wait(Duration::from_secs_f32(1.0));
        // let path = VPathBuilder::start(vec3(0.0, 0.0, 0.0))
        //     .line_to(vec3(100.0, 200.0, 0.0))
        //     .line_to(vec3(-100.0, -100.0, 0.0))
        //     .line_to(vec3(0.0, 0.0, 0.0))
        //     .close().build();
        // app.insert(path);
    }
}

fn main() {
    #[cfg(debug_assertions)]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=trace")).init();
    #[cfg(not(debug_assertions))]
    env_logger::Builder::from_env(Env::default().default_filter_or("test_scene=info,ranim=trace"))
        .init();

    let mut scene = TestScene::default();
    let mut app = RanimRenderApp::new(AppOptions::default());
    scene.construct(&mut app);
    // scene
}
