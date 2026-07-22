use std::time::Instant;

use itertools::Itertools;
use ranim::{
    color::palettes::manim,
    glam::{DVec3, usizevec3},
    items::vitem::{
        TypstText, VItem,
        typst::{CompileOptions, compile, compile_with_options},
    },
    prelude::*,
};
use ranim_anims::morph::MorphAnim;
use ranim_core::animation::StaticAnim;
use ranim::utils::rate_functions::smooth;

const BEHAVIOR_CELLS: &[&str] = &[
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Text and shaping]\
Latin: office affinity AVATAR\
Unicode: 中文排版 · Ελληνικά · العربية\
Styles: #strong[bold] / #emph[italic] / #smallcaps[Small Caps]
"#,
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Mathematics]\
$ integral_0^infinity e^(-x^2) dif x = sqrt(pi) / 2 $\
$ sum_(k=1)^n k^3 = (n(n+1)/2)^2 $\
$ mat(1, 2; 3, 4) vec(x, y) = vec(5, 11) $
"#,
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Paint and outlines]\
#text(fill: rgb("fd971f"), stroke: 0.35pt + white)[filled + stroked text]\
#box(width: 38pt, height: 13pt, fill: rgb("ae81ff"), stroke: 1pt + white)[]
#h(8pt)
#circle(radius: 7pt, fill: rgb("f92672"), stroke: 1pt + white)
#h(8pt)
#line(length: 34pt, stroke: 2pt + rgb("66d9ef"))
"#,
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Transforms and paths]\
#rotate(12deg, text(fill: rgb("fd971f"))[rotated])
#h(14pt)
#scale(x: 85%, y: 125%, text(fill: rgb("66d9ef"))[scaled])\
#polygon(
  fill: rgb("f92672"), stroke: 0.8pt + white,
  (0pt, 10pt), (8pt, 0pt), (16pt, 10pt), (12pt, 20pt), (4pt, 20pt),
)
"#,
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Layout]\
#block(width: 190pt)[This paragraph wraps across multiple lines and checks spaces,
punctuation, baseline placement, and repeated glyph positioning. 0123456789]
"#,
    r#"
#set page(width: auto, height: auto, margin: 2pt)
#set text(size: 11pt, fill: white)
#text(weight: "bold", fill: rgb("a6e22e"))[Known fallbacks]\
Gradient becomes white: #box(width: 42pt, height: 9pt,
  fill: gradient.linear(rgb("f92672"), rgb("66d9ef")))[]\
Clip currently warns: #box(clip: true, width: 65pt, height: 12pt)[overflowing text]
"#,
];

const HOT_CELL: &str = r#"
#set page(width: auto, height: auto, margin: 1pt)
#set text(size: 10pt, fill: white)
$ pi = 3.141592653589793 $
office affinity efficient ffi 0123456789
"#;

fn fit_to_frame(items: &mut Vec<VItem>, height: f64) {
    items
        .scale_to(ScaleHint::PorportionalY(height))
        .move_to(DVec3::ZERO);
}

#[scene(clear_color = "#080a0f")]
#[output(dir = "./output/typst_stress")]
fn typst_behavior(r: &mut RanimScene) {
    let options = CompileOptions {
        include_page_fill: false,
        ..CompileOptions::default()
    };
    let mut cells = BEHAVIOR_CELLS
        .iter()
        .map(|source| {
            let output = compile_with_options(source, options).expect("behavior cell must compile");
            if !output.compiler_warnings.is_empty() {
                eprintln!("Typst compiler warnings: {:#?}", output.compiler_warnings);
            }
            if !output.conversion_warnings.is_empty() {
                eprintln!(
                    "Typst conversion warnings: {:#?}",
                    output.conversion_warnings
                );
            }
            output.document.into_vitems().with(|items| {
                items.scale_to_min(&[
                    ScaleHint::PorportionalX(6.0),
                    ScaleHint::PorportionalY(1.55),
                ]);
            })
        })
        .collect::<Vec<_>>();
    cells
        .arrange_in_grid(
            usizevec3(2, 3, usize::MAX),
            DVec3::new(6.7, 2.0, 0.0),
            DVec3::ZERO,
        )
        .move_to(DVec3::ZERO);
    let matrix = cells.into_iter().flatten().collect::<Vec<_>>();

    let mut title = TypstText::new(
        r#"#text(size: 20pt, weight: "bold", fill: rgb("66d9ef"))[Typst behavior matrix]"#,
    );
    title
        .scale_to(ScaleHint::PorportionalY(0.35))
        .move_to(DVec3::Y * 3.7);

    let mut from = TypstText::new(r#"$ sum_(k=1)^n k = n(n+1)/2 $"#);
    let mut to = TypstText::new(r#"$ sum_(k=1)^n k^3 = (n(n+1)/2)^2 $"#);
    from.scale_to(ScaleHint::PorportionalY(0.55))
        .move_to(DVec3::NEG_Y * 3.95)
        .set_fill_color(manim::YELLOW_C);
    to.scale_to(ScaleHint::PorportionalY(0.55))
        .move_to(DVec3::NEG_Y * 3.95)
        .set_fill_color(manim::GREEN_C);

    let mut morph_sequence = AnimSequence::new();
    morph_sequence
        .forward(0.5)
        .push(
            from.morph_to(to.clone())
                .with_duration(2.0)
                .with_rate_func(smooth),
        )
        .hold(0.5);
    let total_secs = morph_sequence.cursor_sec();

    r.play(CameraFrame::default().show().with_duration(total_secs));
    r.play(stack![
        stack![matrix.show(), title.show()].with_duration(total_secs),
        morph_sequence,
    ]);
    r.insert_time_mark(0.25, TimeMark::Capture("preview-behavior.png".to_owned()));
}

#[scene(clear_color = "#080a0f")]
#[output(dir = "./output/typst_stress")]
fn typst_pressure(r: &mut RanimScene) {
    let columns = 12;
    let rows = 10;
    let compile_started = Instant::now();

    let mut cells = (0..rows)
        .cartesian_product(0..columns)
        .map(|(row, column)| {
            let source = if (row + column) % 2 == 0 {
                HOT_CELL.to_owned()
            } else {
                format!(
                    r#"
#set page(width: auto, height: auto, margin: 1pt)
#set text(size: 10pt, fill: rgb("{:02x}{:02x}{:02x}"))
$ sum_(k=1)^{} k^{} = {} $
cell [{:02}, {:02}] office affine ffi 0123456789
"#,
                    96 + row * 10,
                    128 + column * 8,
                    224 - row * 8,
                    row + column + 4,
                    column % 4 + 1,
                    row * columns + column,
                    row,
                    column,
                )
            };
            compile(&source)
                .expect("stress cell must compile")
                .document
                .into_vitems()
                .with(|items| {
                    items.scale_to_min(&[
                        ScaleHint::PorportionalX(1.1),
                        ScaleHint::PorportionalY(0.48),
                    ]);
                })
        })
        .collect::<Vec<_>>();
    let vitem_count = cells.iter().map(Vec::len).sum::<usize>();
    eprintln!(
        "Typst pressure: compiled {} cells ({} hot, {} distinct) into {} VItems in {:?}",
        rows * columns,
        rows * columns / 2,
        rows * columns / 2,
        vitem_count,
        compile_started.elapsed(),
    );

    cells.arrange_in_grid(
        usizevec3(columns, rows, usize::MAX),
        DVec3::new(1.25, 0.78, 0.0),
        DVec3::splat(0.08),
    );
    let mut items = cells.into_iter().flatten().collect::<Vec<_>>();
    fit_to_frame(&mut items, 7.5);

    let target = items.clone().with(|items| {
        items.apply_point_func(|point| {
            point.x += 0.08 * (point.y * 2.2).sin();
            point.y += 0.05 * (point.x * 2.7).cos();
        });
    });

    let mut content = AnimSequence::new();
    content
        .push(items.clone().show())
        .hold(0.5)
        .push(
            items
                .morph_to(target.clone())
                .with_duration(3.0)
                .with_rate_func(smooth),
        )
        .hold(0.5);
    r.play(
        CameraFrame::default()
            .show()
            .with_duration(content.cursor_sec()),
    );
    r.play(content);
    r.insert_time_mark(0.25, TimeMark::Capture("preview-pressure.png".to_owned()));
}
