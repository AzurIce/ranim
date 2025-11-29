use std::any::Any;

use crate::{
    Extract,
    animation::{AnimationCell, CoreItemAnimation, Eval, Static},
    core_item::{AnyExtractCoreItem, CoreItem, DynItem},
};

/// A timeline for a animations.
#[derive(Default)]
pub struct NeoItemTimeline {
    anims: Vec<Box<dyn CoreItemAnimation>>,
    // Followings are states use while constructing
    cur_sec: f64,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,
}

impl NeoItemTimeline {
    /// Create a new timeline.
    pub fn new() -> Self {
        Self::default()
    }
    /// Show the item.
    ///
    /// This will start planning an static anim if there isn't an planning static anim.
    pub fn show(&mut self) -> &mut Self {
        if self.planning_static_start_sec.is_none() {
            self.planning_static_start_sec = Some(self.cur_sec)
        }
        self
    }
    /// Hide the item.
    ///
    /// This will submit a static anim if there is an planning static anim.
    pub fn hide(&mut self) -> &mut Self {
        // println!("hide");
        self._submit_planning_static_anim();
        self
    }
    /// Forward the timeline by `secs`
    pub fn forward(&mut self, secs: f64) -> &mut Self {
        self.cur_sec += secs;
        self
    }
    /// Forward the timeline to `target_sec` if the current sec is smaller than it.
    pub fn forward_to(&mut self, target_sec: f64) -> &mut Self {
        if target_sec > self.cur_sec {
            self.forward(target_sec - self.cur_sec);
        }
        self
    }
    fn _submit_planning_static_anim(&mut self) -> bool {
        // println!("{:?}", self.planning_static_start_sec);
        if let (Some(start), Some(last_anim)) =
            (self.planning_static_start_sec.take(), self.anims.last())
        {
            let state = last_anim.eval_alpha_dyn(1.0);
            self.anims.push(Box::new(
                Static(state)
                    .into_animation_cell()
                    .at(start)
                    .with_duration(self.cur_sec - start)
                    .with_enabled(last_anim.anim_info().enabled),
            ));
            return true;
        }
        false
    }
    // /// Plays an anim with `anim_func`.
    // pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationCell<T>) -> &mut Self {
    //     self.play(anim_func(self.state.clone()))
    // }
    /// Plays an anim.
    pub fn play<T: AnyExtractCoreItem>(&mut self, anim: AnimationCell<T>) -> &mut Self {
        self._submit_planning_static_anim();
        // let res = anim.eval_alpha(1.0);
        let duration = anim.info.duration_secs;
        self.anims
            .push(Box::new(anim.at(self.cur_sec).with_duration(duration)));
        self.cur_sec += duration;
        // self.update(res);
        self.show();
        self
    }
    /// Evaluate the state at `alpha`
    pub fn eval_at_alpha(&self, alpha: f64) -> Option<(DynItem, u64)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        self.eval_at_sec(alpha * (end - start) + start)
    }
    /// Evaluate the state at `target_sec`
    pub fn eval_at_sec(&self, target_sec: f64) -> Option<(DynItem, u64)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };

        if !(start..=end).contains(&target_sec) {
            return None;
        }

        self.anims
            .iter()
            .enumerate()
            .filter(|(_, a)| a.anim_info().enabled)
            .find_map(|(idx, anim)| {
                let range = anim.anim_info().range();
                if range.contains(&target_sec)
                    || (idx == self.anims.len() - 1 && target_sec == range.end)
                {
                    Some((idx, anim, range))
                } else {
                    None
                }
            })
            .map(|(idx, anim, range)| {
                let alpha = (target_sec - range.start) / (range.end - range.start);
                (anim.eval_alpha_dyn(alpha), idx as u64)
            })
    }
}

impl TimelineFunc for NeoItemTimeline {
    fn start_sec(&self) -> Option<f64> {
        self.anims.first().map(|a| a.anim_info().range().start)
    }
    fn end_sec(&self) -> Option<f64> {
        self.anims.last().map(|a| a.anim_info().range().end)
    }
    fn seal(&mut self) {
        self._submit_planning_static_anim();
    }
    fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    fn forward(&mut self, duration_secs: f64) {
        self.cur_sec += duration_secs;
    }
    fn show(&mut self) {
        self.show();
    }
    fn hide(&mut self) {
        self.hide();
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        // self.inner
        //     .iter()
        //     .flat_map(|timeline| timeline.get_animation_infos())
        //     .collect()
        vec![]
    }
    fn type_name(&self) -> &str {
        ""
        // self.get_dyn().type_name()
    }
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(Vec<CoreItem>, u64)> {
        self.eval_at_sec(target_sec).map(|(dyn_item, idx)| {
            let mut items = Vec::new();
            dyn_item.extract_into(&mut items);
            (items, idx)
        })
    }
}

// MARK: TimelineFunc
/// Functions for a timeline
pub trait TimelineFunc: Any {
    /// The start sec of the timeline(the start sec of the first animation.)
    fn start_sec(&self) -> Option<f64>;
    /// The end sec of the timeline(the end sec of the last animation.)
    fn end_sec(&self) -> Option<f64>;
    /// The range of the timeline.
    fn range_sec(&self) -> Option<std::ops::Range<f64>> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        Some(start..end)
    }
    /// Seal the timeline func(submit the planning static anim)
    fn seal(&mut self);
    /// The current sec of the timeline.
    fn cur_sec(&self) -> f64;
    /// Forward the timeline by `secs`
    fn forward(&mut self, secs: f64);
    /// Forward the timeline to `target_sec`
    fn forward_to(&mut self, target_sec: f64) {
        let duration = target_sec - self.cur_sec();
        if duration > 0.0 {
            self.forward(duration);
        }
    }
    /// Show the item
    fn show(&mut self);
    /// Hide the item
    fn hide(&mut self);
    /// Get the animation infos
    fn get_animation_infos(&self) -> Vec<AnimationInfo>;
    /// The type name of the timeline
    fn type_name(&self) -> &str;

    // fn eval_sec_any(&self, target_sec: f64) -> Option<(EvalResult<dyn Any>, usize)>;
    /// Evaluate timeline's primitives at target sec
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(Vec<CoreItem>, u64)>;
}

// MARK: TimelinesFunc
/// Functions for timelines
pub trait TimelinesFunc {
    /// Seal timelines
    fn seal(&mut self);
    /// Get the max end_sec of the timelines
    fn max_total_secs(&self) -> f64;
    /// Sync the timelines
    fn sync(&mut self);
    /// Forward all timelines by `sec`
    fn forward(&mut self, secs: f64);
    /// Forward all timelines to `target_sec`
    fn forward_to(&mut self, target_sec: f64);
}

impl<I: ?Sized, T: TimelineFunc> TimelinesFunc for I
where
    for<'a> &'a mut I: IntoIterator<Item = &'a mut T>,
    for<'a> &'a I: IntoIterator<Item = &'a T>,
{
    fn seal(&mut self) {
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.seal();
        });
    }
    fn max_total_secs(&self) -> f64 {
        self.into_iter()
            .map(|timeline: &T| timeline.cur_sec())
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
    }
    fn sync(&mut self) {
        let max_elapsed_secs = self.max_total_secs();
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.forward_to(max_elapsed_secs);
        });
    }
    fn forward(&mut self, secs: f64) {
        self.into_iter()
            .for_each(|timeline: &mut T| timeline.forward(secs));
    }
    fn forward_to(&mut self, target_sec: f64) {
        self.into_iter().for_each(|timeline: &mut T| {
            timeline.forward_to(target_sec);
        });
    }
}

/// Info of an animation
pub struct AnimationInfo {
    /// The name of the animation
    pub anim_name: String,
    /// The time range of the animation
    pub range: std::ops::Range<f64>,
}
