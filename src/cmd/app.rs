//! Read-only scene browser. Scene construction is lazy and runs off the UI thread.
//!
//! This inspects the runtime hierarchy and registered type metadata, not arbitrary
//! evaluator fields. Registration alone does not imply KDL serialization support.

use std::{
    collections::BTreeMap,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

use eframe::egui;
use ranim_core::animation::node::{AnimationInfo, AnimationInfoKind};

use crate::{AnimationRegistry, AppExt, RanimApp, Scene, SceneConstructor};

struct SceneSnapshot {
    duration: f64,
    roots: Vec<AnimationInfo>,
    nodes: usize,
    build_time: Duration,
}

impl SceneSnapshot {
    fn build(scene: &Scene) -> Result<Self, String> {
        catch_unwind(AssertUnwindSafe(|| {
            let start = Instant::now();
            let sealed = scene.build_scene();
            let roots = sealed.get_animation_infos();
            let nodes = count_nodes(&roots);
            Self {
                duration: sealed.total_secs(),
                roots,
                nodes,
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

fn count_nodes(roots: &[AnimationInfo]) -> usize {
    roots
        .iter()
        .map(|node| 1 + count_nodes(&node.children))
        .sum()
}

fn node_at_path<'a>(roots: &'a [AnimationInfo], path: &[usize]) -> Option<&'a AnimationInfo> {
    let (index, rest) = path.split_first()?;
    let node = roots.get(*index)?;
    if rest.is_empty() {
        Some(node)
    } else {
        node_at_path(&node.children, rest)
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

fn kind_name(kind: AnimationInfoKind) -> &'static str {
    match kind {
        AnimationInfoKind::Anim => "Animation",
        AnimationInfoKind::Children => "Container",
        AnimationInfoKind::Audio => "Audio",
    }
}

/// An inspector for the scenes and animation registrations of an assembled App.
pub struct InspectorApp {
    // Keep project resources alive for captured scene constructors.
    _project: RanimApp,
    scenes: Vec<Scene>,
    registry: AnimationRegistry,
    filter: String,
    selected_scene: Option<usize>,
    selected_node: Vec<usize>,
    snapshots: BTreeMap<usize, Result<SceneSnapshot, String>>,
    worker: Option<(usize, Receiver<Result<SceneSnapshot, String>>)>,
    expand_all: Option<bool>,
}

impl InspectorApp {
    /// Create an inspector without building any scenes or initializing a renderer.
    pub fn new(project: RanimApp) -> Self {
        let scenes = project.scenes().to_vec();
        let registry = project
            .world()
            .get_resource::<AnimationRegistry>()
            .cloned()
            .unwrap_or_default();
        Self {
            _project: project,
            scenes,
            registry,
            filter: String::new(),
            selected_scene: None,
            selected_node: Vec::new(),
            snapshots: BTreeMap::new(),
            worker: None,
            expand_all: None,
        }
    }

    fn update_worker(&mut self, ctx: &egui::Context) {
        if let Some((index, receiver)) = &self.worker {
            match receiver.try_recv() {
                Ok(result) => {
                    self.snapshots.insert(*index, result);
                    self.worker = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.snapshots
                        .insert(*index, Err("Scene worker disconnected".into()));
                    self.worker = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        // At most one constructor runs at a time, even when selection changes
        // rapidly. Finished snapshots are reused until explicitly rebuilt.
        if self.worker.is_none()
            && let Some(index) = self.selected_scene
            && !self.snapshots.contains_key(&index)
        {
            let scene = self.scenes[index].clone();
            let (sender, receiver) = mpsc::channel();
            let repaint = ctx.clone();
            match std::thread::Builder::new()
                .name("ranim-inspector".into())
                .spawn(move || {
                    let _ = sender.send(SceneSnapshot::build(&scene));
                    repaint.request_repaint();
                }) {
                Ok(_) => self.worker = Some((index, receiver)),
                Err(error) => {
                    self.snapshots.insert(index, Err(error.to_string()));
                }
            }
        }
        if self.worker.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn show(&mut self, ui: &mut egui::Ui) {
        self.update_worker(ui.ctx());
        egui::Panel::top("inspector-toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Ranim Inspector");
                ui.separator();
                ui.label(format!("{} scenes", self.scenes.len()));
                ui.label(format!(
                    "{} registered animation families",
                    self.registry.iter().len()
                ));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("Read-only · no serialization codecs");
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

        egui::Panel::right("inspector-properties").default_size(340.0).min_size(230.0).resizable(true).show(ui, |ui| {
            ui.heading("Inspector");
            egui::ScrollArea::vertical().id_salt("properties").show(ui, |ui| {
                let node = self.selected_scene
                    .and_then(|index| self.snapshots.get(&index))
                    .and_then(|result| result.as_ref().ok())
                    .and_then(|snapshot| node_at_path(&snapshot.roots, &self.selected_node));
                if let Some(node) = node {
                    show_node_details(ui, node, &self.selected_node, &self.registry);
                } else {
                    ui.weak("Select an animation node to inspect it.");
                }
                ui.separator();
                egui::CollapsingHeader::new("Registered animation families").show(ui, |ui| {
                    if self.registry.iter().len() == 0 {
                        ui.weak("Add BuiltinAnimationsPlugin or register custom animations in a Plugin.");
                    }
                    for descriptor in self.registry.iter() {
                        ui.label(descriptor.id()).on_hover_text(format!("{} {}\n{}", descriptor.package, descriptor.version, descriptor.rust_path));
                    }
                });
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
                if ui
                    .add_enabled(self.worker.is_none(), egui::Button::new("Rebuild scene"))
                    .clicked()
                {
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
                        snapshot.duration,
                        snapshot.roots.len(),
                        snapshot.nodes,
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
                                    &snapshot.roots,
                                    &mut Vec::new(),
                                    index,
                                    open,
                                );
                            }
                            show_tree(
                                ui,
                                &snapshot.roots,
                                &mut Vec::new(),
                                &mut self.selected_node,
                                &self.registry,
                                index,
                            );
                        });
                    if snapshot.roots.is_empty() {
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
    nodes: &[AnimationInfo],
    path: &mut Vec<usize>,
    selected: &mut Vec<usize>,
    registry: &AnimationRegistry,
    scene: usize,
) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        let registered = registry.resolve(&node.anim_name).is_some();
        let label = format!(
            "{}  [{:.3} … {:.3}]{}{}",
            short_name(&node.anim_name),
            node.range.start,
            node.range.end,
            if node.enabled { "" } else { " · disabled" },
            if registered { "" } else { " · unregistered" }
        );
        if node.children.is_empty() {
            if ui
                .selectable_label(*selected == *path, label)
                .on_hover_text(&node.anim_name)
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
                        .on_hover_text(&node.anim_name)
                        .clicked()
                    {
                        *selected = path.clone();
                    }
                })
                .body(|ui| {
                    show_tree(ui, &node.children, path, selected, registry, scene);
                });
        }
        path.pop();
    }
}

fn set_descendant_expansion(
    ui: &egui::Ui,
    nodes: &[AnimationInfo],
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
        set_descendant_expansion(ui, &node.children, path, scene, open);
        path.pop();
    }
}

fn property(ui: &mut egui::Ui, label: &str, value: impl Into<String>) {
    ui.weak(label);
    ui.label(value.into());
    ui.end_row();
}

fn show_node_details(
    ui: &mut egui::Ui,
    node: &AnimationInfo,
    path: &[usize],
    registry: &AnimationRegistry,
) {
    ui.heading(short_name(&node.anim_name));
    ui.label(egui::RichText::new(&node.anim_name).monospace().small());
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
            property(ui, "Kind", kind_name(node.kind));
            property(ui, "Enabled", node.enabled.to_string());
            property(ui, "Start (parent s)", format!("{:.6}", node.range.start));
            property(ui, "End (parent s)", format!("{:.6}", node.range.end));
            property(
                ui,
                "Duration (s)",
                format!("{:.6}", node.range.end - node.range.start),
            );
            property(
                ui,
                "Content duration (s)",
                format!("{:.6}", node.content_duration_secs),
            );
            property(ui, "Children", node.children.len().to_string());
            property(ui, "Descendants", count_nodes(&node.children).to_string());
            property(
                ui,
                "Simulation step",
                node.sim_step
                    .map_or_else(|| "—".into(), |step| format!("{step:.8} progress")),
            );
        });
    ui.separator();
    if let Some(descriptor) = registry.resolve(&node.anim_name) {
        ui.label("Registered animation family");
        ui.monospace(descriptor.id());
        ui.label(format!("{} {}", descriptor.package, descriptor.version));
    } else {
        ui.label("Unregistered runtime type");
        ui.weak("This node can run and be inspected, but has no registered family in this App. Closure adapters and captured holds are runtime-only unless wrapped by a named definition.");
    }
    ui.weak("Type registration is metadata only; it does not guarantee that parameters can be serialized.");
    ui.separator();
    ui.label("Rate function · local progress → content progress");
    show_rate_curve(ui, node.rate_func);
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
    use ranim_core::animation::{build::PlaybackExt, eval::pure::Pure};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[test]
    fn browsing_is_lazy_and_snapshots_preserve_unregistered_nodes() {
        let builds = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&builds);
        let mut project = App::new();
        project.add_scene(Scene::new("sample", move |r: &mut RanimScene| {
            counter.fetch_add(1, Ordering::Relaxed);
            r.play(Pure::new(|alpha| alpha).with_duration(2.0));
        }));
        let app = InspectorApp::new(project.build());
        assert_eq!(builds.load(Ordering::Relaxed), 0);
        let snapshot = SceneSnapshot::build(&app.scenes[0]).unwrap();
        assert_eq!(snapshot.duration, 2.0);
        assert_eq!(snapshot.nodes, 1);
        assert!(app.registry.resolve(&snapshot.roots[0].anim_name).is_none());
        assert!(node_at_path(&snapshot.roots, &[0]).is_some());
        assert!(node_at_path(&snapshot.roots, &[0, 0]).is_none());
        assert!(node_at_path(&snapshot.roots, &[]).is_none());
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
        assert!(inspector.worker.is_none());
        inspector.expand_all = Some(false);
        egui::__run_test_ui(|ui| inspector.show(ui));
    }
}
