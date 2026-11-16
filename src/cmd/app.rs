//! Read-only scene browser. Definitions are constructed lazily on the UI thread.
//!
//! This inspects retained definition trees and playback settings, not arbitrary
//! input or recipe fields. It does not provide scene serialization.

use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    time::{Duration, Instant},
};

use eframe::egui;
use ranim_core::{RanimScene, animation::def_node::AnimDefNode};

use crate::{AppExt, RanimApp, Scene, SceneConstructor};

struct SceneSnapshot {
    scene: RanimScene,
    build_time: Duration,
}

impl SceneSnapshot {
    fn build(scene: &Scene) -> Result<Self, String> {
        catch_unwind(AssertUnwindSafe(|| {
            let start = Instant::now();
            let definition = scene.build_definition();
            Self {
                scene: definition,
                build_time: start.elapsed(),
            }
        }))
        .map_err(|payload| {
            payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_else(|| "Scene constructor panicked with a non-string payload".into())
        })
    }
}

fn count_nodes(roots: &[AnimDefNode]) -> usize {
    roots
        .iter()
        .map(|node| 1 + count_nodes(node.definitions()))
        .sum()
}

fn node_at_path<'a>(roots: &'a [AnimDefNode], path: &[usize]) -> Option<&'a AnimDefNode> {
    let (index, rest) = path.split_first()?;
    let node = roots.get(*index)?;
    if rest.is_empty() {
        Some(node)
    } else {
        node_at_path(node.definitions(), rest)
    }
}

fn short_name(name: &str) -> &str {
    name.split('<')
        .next()
        .unwrap_or(name)
        .rsplit("::")
        .next()
        .unwrap_or(name)
}

fn kind_name(node: &AnimDefNode) -> &'static str {
    if node.is_container() {
        "Container"
    } else if node.is_audio() {
        "Audio"
    } else {
        "Animation"
    }
}

/// An inspector for the scenes and retained animation definitions of an assembled App.
pub struct InspectorApp {
    // Keep project resources alive for captured scene constructors.
    _project: RanimApp,
    scenes: Vec<Scene>,
    filter: String,
    selected_scene: Option<usize>,
    selected_node: Vec<usize>,
    snapshots: BTreeMap<usize, Result<SceneSnapshot, String>>,
    expand_all: Option<bool>,
}

impl InspectorApp {
    /// Create an inspector without building any scenes or initializing a renderer.
    pub fn new(project: RanimApp) -> Self {
        let scenes = project.scenes().to_vec();
        Self {
            _project: project,
            scenes,
            filter: String::new(),
            selected_scene: None,
            selected_node: Vec::new(),
            snapshots: BTreeMap::new(),
            expand_all: None,
        }
    }

    fn build_selected_scene(&mut self) {
        if let Some(index) = self.selected_scene
            && !self.snapshots.contains_key(&index)
        {
            self.snapshots
                .insert(index, SceneSnapshot::build(&self.scenes[index]));
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        self.build_selected_scene();
        egui::Panel::top("inspector-toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Ranim Inspector");
                ui.separator();
                ui.label(format!("{} scenes", self.scenes.len()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("Read-only");
                });
            });
        });

        egui::Panel::left("inspector-scenes")
            .default_size(230.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.heading("Scenes");
                ui.add(egui::TextEdit::singleline(&mut self.filter).hint_text("Filter scenes…"));
                ui.separator();
                let filter = self.filter.to_lowercase();
                egui::ScrollArea::vertical()
                    .id_salt("scene-list")
                    .show(ui, |ui| {
                        for (index, scene) in self.scenes.iter().enumerate() {
                            if !scene.name.to_lowercase().contains(&filter) {
                                continue;
                            }
                            if ui
                                .selectable_label(self.selected_scene == Some(index), &scene.name)
                                .clicked()
                            {
                                self.selected_scene = Some(index);
                                self.selected_node.clear();
                                ui.ctx().request_repaint();
                            }
                        }
                        if self.scenes.is_empty() {
                            ui.weak("No scenes registered in this App.");
                        }
                    });
            });

        egui::Panel::right("inspector-properties")
            .default_size(340.0)
            .min_size(230.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.heading("Inspector");
                egui::ScrollArea::vertical()
                    .id_salt("properties")
                    .show(ui, |ui| {
                        let node = self
                            .selected_scene
                            .and_then(|index| self.snapshots.get(&index))
                            .and_then(|result| result.as_ref().ok())
                            .and_then(|snapshot| {
                                node_at_path(snapshot.scene.root.definitions(), &self.selected_node)
                            });
                        if let Some(node) = node {
                            show_node_details(ui, node, &self.selected_node);
                        } else {
                            ui.weak("Select an animation node to inspect it.");
                        }
                    });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let Some(index) = self.selected_scene else {
                ui.centered_and_justified(|ui| {
                    ui.weak(
                        "Select a scene from the left to build and inspect its animation tree.",
                    );
                });
                return;
            };
            ui.heading(&self.scenes[index].name);
            ui.horizontal(|ui| {
                if ui.button("Rebuild scene").clicked() {
                    self.snapshots.remove(&index);
                    self.selected_node.clear();
                    ui.ctx().request_repaint();
                }
                if ui.button("Expand all").clicked() {
                    self.expand_all = Some(true);
                }
                if ui.button("Collapse all").clicked() {
                    self.expand_all = Some(false);
                }
            });
            ui.separator();
            match self.snapshots.get(&index) {
                Some(Ok(snapshot)) => {
                    ui.label(format!(
                        "{:.3} s · {} root nodes · {} total nodes · built in {:.2?}",
                        snapshot.scene.root.duration_secs(),
                        snapshot.scene.root.definitions().len(),
                        count_nodes(snapshot.scene.root.definitions()),
                        snapshot.build_time
                    ));
                    ui.weak(
                        "Ranges are in the parent's local content time, not global scene time.",
                    );
                    egui::ScrollArea::both()
                        .id_salt(("animation-tree", index))
                        .show(ui, |ui| {
                            if let Some(open) = self.expand_all {
                                set_descendant_expansion(
                                    ui,
                                    snapshot.scene.root.definitions(),
                                    &mut Vec::new(),
                                    index,
                                    open,
                                );
                            }
                            show_tree(
                                ui,
                                snapshot.scene.root.definitions(),
                                &mut Vec::new(),
                                &mut self.selected_node,
                                index,
                            );
                        });
                    if snapshot.scene.root.definitions().is_empty() {
                        ui.weak("This scene has no animation nodes.");
                    }
                }
                Some(Err(error)) => {
                    ui.colored_label(ui.visuals().error_fg_color, "Scene build failed");
                    ui.label(error);
                    ui.weak("Other scenes remain available. Fix the scene and rebuild to retry.");
                }
                None => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("Waiting for scene construction…");
                    });
                }
            }
            self.expand_all = None;
        });
    }
}

impl eframe::App for InspectorApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }
}

fn show_tree(
    ui: &mut egui::Ui,
    nodes: &[AnimDefNode],
    path: &mut Vec<usize>,
    selected: &mut Vec<usize>,
    scene: usize,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let range = node.time_range();
        let label = format!(
            "{}  [{:.3} … {:.3}]{}",
            short_name(node.anim_name()),
            range.start,
            range.end,
            if node.enabled() { "" } else { " · disabled" }
        );
        if node.definitions().is_empty() {
            if ui
                .selectable_label(*selected == *path, label)
                .on_hover_text(node.anim_name())
                .clicked()
            {
                *selected = path.clone();
            }
        } else {
            let id = egui::Id::new(("animation-node", scene, &*path));
            let state = egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                id,
                path.len() < 3,
            );
            state
                .show_header(ui, |ui| {
                    if ui
                        .selectable_label(*selected == *path, label)
                        .on_hover_text(node.anim_name())
                        .clicked()
                    {
                        *selected = path.clone();
                    }
                })
                .body(|ui| {
                    show_tree(ui, node.definitions(), path, selected, scene);
                });
        }
        path.pop();
    }
}

fn set_descendant_expansion(
    ui: &egui::Ui,
    nodes: &[AnimDefNode],
    path: &mut Vec<usize>,
    scene: usize,
    open: bool,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let id = egui::Id::new(("animation-node", scene, &*path));
        let mut state =
            egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, open);
        state.set_open(open);
        state.store(ui.ctx());
        set_descendant_expansion(ui, node.definitions(), path, scene, open);
        path.pop();
    }
}

fn property(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    ui.weak(label);
    ui.label(value.into());
    ui.end_row();
}

fn show_node_details(ui: &mut egui::Ui, node: &AnimDefNode, path: &[usize]) {
    let range = node.time_range();
    ui.heading(short_name(node.anim_name()));
    ui.label(egui::RichText::new(node.anim_name()).monospace().small());
    ui.separator();
    egui::Grid::new("node-properties")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            property(
                ui,
                "Path",
                path.iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("/"),
            );
            property(ui, "Kind", kind_name(node));
            property(ui, "Enabled", node.enabled().to_string());
            property(ui, "Start (parent s)", format!("{:.6}", range.start));
            property(ui, "End (parent s)", format!("{:.6}", range.end));
            property(ui, "Duration (s)", format!("{:.6}", node.duration_secs()));
            property(
                ui,
                "Content duration (s)",
                format!("{:.6}", node.content_duration_secs()),
            );
            property(ui, "Children", node.definitions().len().to_string());
            property(
                ui,
                "Descendants",
                count_nodes(node.definitions()).to_string(),
            );
            property(
                ui,
                "Simulation step",
                node.sim_step()
                    .map_or_else(|| "—".into(), |step| format!("{step:.8} progress")),
            );
        });
    ui.separator();
    ui.label("Rate function · local progress → content progress");
    show_rate_curve(ui, node.rate_func());
}

fn show_rate_curve(ui: &mut egui::Ui, rate: fn(f64) -> f64) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 120.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let plot = rect.shrink(8.0);
    let points: Result<Vec<_>, _> = catch_unwind(|| {
        (0..=64)
            .map(|index| {
                let x = f64::from(index) / 64.0;
                (x, rate(x))
            })
            .collect()
    });
    let Ok(points) = points else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Rate function panicked",
            egui::FontId::default(),
            ui.visuals().error_fg_color,
        );
        return;
    };
    let min = points
        .iter()
        .filter_map(|(_, y)| y.is_finite().then_some(*y))
        .fold(0.0, f64::min);
    let max = points
        .iter()
        .filter_map(|(_, y)| y.is_finite().then_some(*y))
        .fold(1.0, f64::max);
    let mut previous = None;
    for (x, y) in points {
        if !y.is_finite() {
            previous = None;
            continue;
        }
        let point = egui::pos2(
            plot.left() + plot.width() * x as f32,
            plot.bottom() - plot.height() * ((y - min) / (max - min)) as f32,
        );
        if let Some(previous) = previous {
            painter.line_segment(
                [previous, point],
                egui::Stroke::new(1.5, ui.visuals().selection.bg_fill),
            );
        }
        previous = Some(point);
    }
}

/// Launch a native egui inspector for all scenes registered in the project.
pub fn run_app(project: RanimApp) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 800.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "Ranim Inspector",
        options,
        Box::new(move |_| Ok(Box::new(InspectorApp::new(project)))),
    )
    .map_err(|error| anyhow::anyhow!("failed to launch scene inspector: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{App, RanimAppExt, RanimScene};
    use ranim_core::animation::{
        AnimRecipe,
        build::PlaybackExt,
        def_node::{AnimDef, AnimDefNodeContent, ItemAnimExt},
        eval::{Static, pure::Pure},
        node::AnimNode,
    };
    use std::rc::Rc;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn browsing_is_lazy_and_preserves_non_send_inputs() {
        struct Recipe;
        impl AnimRecipe<Rc<f64>> for Recipe {
            fn build(&self, input: &Rc<f64>) -> AnimNode {
                AnimNode::from_eval(Static(**input), 1.0)
            }
        }
        let builds = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&builds);
        let mut project = App::new();
        project.add_scene(Scene::new("sample", move |r: &mut RanimScene| {
            counter.fetch_add(1, Ordering::Relaxed);
            r.play(Rc::new(1.0).anim(Recipe).with_duration(2.0));
        }));
        let mut app = InspectorApp::new(project.build());
        assert_eq!(builds.load(Ordering::Relaxed), 0);
        app.selected_scene = Some(0);
        app.build_selected_scene();
        app.build_selected_scene();
        assert_eq!(builds.load(Ordering::Relaxed), 1);
        let snapshot = app.snapshots[&0].as_ref().unwrap();
        let roots = snapshot.scene.root.definitions();
        assert_eq!(snapshot.scene.root.duration_secs(), 2.0);
        let AnimDefNodeContent::Leaf(leaf) = &node_at_path(roots, &[0]).unwrap().content else {
            panic!()
        };
        assert_eq!(
            *leaf
                .downcast_ref::<AnimDef<Rc<f64>, Recipe>>()
                .unwrap()
                .initial,
            1.0
        );
        assert!(node_at_path(roots, &[0, 0]).is_none());
        assert!(node_at_path(roots, &[]).is_none());
    }

    #[test]
    fn constructor_panics_become_inspectable_errors() {
        let scene = Scene::new("broken", |_: &mut RanimScene| panic!("broken scene"));
        assert!(matches!(SceneSnapshot::build(&scene), Err(message) if message == "broken scene"));
    }

    #[test]
    fn egui_draws_empty_tree_and_selected_node_without_a_window() {
        let mut app = App::new();
        let mut inspector = InspectorApp::new(app.build());
        egui::__run_test_ui(|ui| inspector.show(ui));

        let mut app = App::new();
        app.add_scene(Scene::new("nested", |r: &mut RanimScene| {
            r.play(ranim_core::seq![
                Pure::new(|alpha| alpha).with_duration(2.0)
            ]);
        }));
        let mut inspector = InspectorApp::new(app.build());
        inspector
            .snapshots
            .insert(0, SceneSnapshot::build(&inspector.scenes[0]));
        inspector.selected_scene = Some(0);
        inspector.selected_node = vec![0, 0];
        inspector.expand_all = Some(true);
        egui::__run_test_ui(|ui| inspector.show(ui));
        inspector.expand_all = Some(false);
        egui::__run_test_ui(|ui| inspector.show(ui));
    }
}
