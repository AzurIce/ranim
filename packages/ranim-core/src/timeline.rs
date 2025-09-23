use std::{any::Any, sync::Arc};

use crate::{
    Extract,
    animation::{AnimationSpan, EvalResult, Evaluator},
    primitives::Primitives,
    utils::calculate_hash,
};

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
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(EvalResult<Primitives>, u64)>;
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

impl TimelineFunc for Box<dyn TimelineFunc> {
    fn start_sec(&self) -> Option<f64> {
        self.as_ref().start_sec()
    }
    fn end_sec(&self) -> Option<f64> {
        self.as_ref().end_sec()
    }
    fn seal(&mut self) {
        self.as_mut().seal()
    }
    fn cur_sec(&self) -> f64 {
        self.as_ref().cur_sec()
    }
    fn forward(&mut self, secs: f64) {
        self.as_mut().forward(secs)
    }
    fn show(&mut self) {
        self.as_mut().show()
    }
    fn hide(&mut self) {
        self.as_mut().hide()
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.as_ref().get_animation_infos()
    }
    fn type_name(&self) -> &str {
        self.as_ref().type_name()
    }
    // fn eval_sec_any(&self, target_sec: f64) -> Option<(EvalResult<dyn Any>, usize)> {
    //     self.as_ref().eval_sec_any(target_sec)
    // }
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(EvalResult<Primitives>, u64)> {
        self.as_ref().eval_primitives_at_sec(target_sec)
    }
}

// MARK: ItemDynTimelines
// ANCHOR: ItemDynTimelines
/// A item timeline which contains multiple `Box<dyn TimelineFunc>`, so
/// that it can contains multiple [`ItemTimeline<T>`] in different type of `T`.
#[derive(Default)]
pub struct ItemDynTimelines {
    inner: Vec<Box<dyn TimelineFunc>>,
}

impl TimelineFunc for ItemDynTimelines {
    fn start_sec(&self) -> Option<f64> {
        self.get_dyn().start_sec()
    }
    fn end_sec(&self) -> Option<f64> {
        self.get_dyn().end_sec()
    }
    fn seal(&mut self) {
        self.get_dyn_mut().seal();
    }
    fn cur_sec(&self) -> f64 {
        self.get_dyn().cur_sec()
    }
    fn forward(&mut self, duration_secs: f64) {
        self.get_dyn_mut().forward(duration_secs);
    }
    fn show(&mut self) {
        self.get_dyn_mut().show();
    }
    fn hide(&mut self) {
        self.get_dyn_mut().hide();
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        self.inner
            .iter()
            .flat_map(|timeline| timeline.get_animation_infos())
            .collect()
    }
    fn type_name(&self) -> &str {
        self.get_dyn().type_name()
    }
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(EvalResult<Primitives>, u64)> {
        self.eval_primitives_at_sec(target_sec)
    }
}

impl ItemDynTimelines {
    /// Create a new [`ItemDynTimelines`]
    pub fn new() -> Self {
        Self::default()
    }
    /// Push a new [`ItemTimeline<T>`] to the end of the timelines
    pub fn push<T: Extract + Clone + 'static>(&mut self, timeline: ItemTimeline<T>) {
        self.inner.push(Box::new(timeline));
    }
    /// Evaluate the timeline and extract to primitives at `alpha`
    pub fn eval_primitives_at_alpha(&self, alpha: f64) -> Option<(EvalResult<Primitives>, u64)> {
        let target_sec = self.inner.max_total_secs() * alpha;
        self.eval_primitives_at_sec(target_sec)
    }
    /// Evaluate the timeline at `target_sec`
    pub fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(EvalResult<Primitives>, u64)> {
        // println!("len: {}", self.timelines.len());
        // println!("ItemDynTimelines::eval_sec_extracted_any: {}", target_sec);

        let (timeline_idx, timeline) = self.inner.iter().enumerate().find(|(idx, timeline)| {
            timeline
                .range_sec()
                .map(|range| {
                    range.contains(&target_sec)
                        || *idx == self.inner.len() - 1 && range.end == target_sec
                })
                .unwrap_or(false)
        })?;

        timeline
            .eval_primitives_at_sec(target_sec)
            .map(|(res, idx)| (res, calculate_hash(&(timeline_idx, idx))))
    }
}

impl ItemDynTimelines {
    /// As a reference of [`TimelineFunc`]
    pub fn get_dyn(&self) -> &dyn TimelineFunc {
        // TODO: make this unwrap better
        self.inner.last().unwrap()
    }
    /// As a mutable reference of [`TimelineFunc`]
    pub fn get_dyn_mut(&mut self) -> &mut dyn TimelineFunc {
        // TODO: make this unwrap better
        self.inner.last_mut().unwrap()
    }
    /// As a reference of [`ItemTimeline<T>`]
    ///
    /// Should make sure that the last timeline in it is of type `T`
    pub fn get<T: 'static>(&self) -> &ItemTimeline<T> {
        // TODO: make this unwrap better
        (self.inner.last().unwrap().as_ref() as &dyn Any)
            .downcast_ref()
            .unwrap()
    }
    /// As a mutable reference of [`ItemTimeline<T>`]
    ///
    /// Should make sure that the last timeline in it is of type `T`
    pub fn get_mut<T: 'static>(&mut self) -> &mut ItemTimeline<T> {
        // TODO: make this unwrap better
        (self.inner.last_mut().unwrap().as_mut() as &mut dyn Any)
            .downcast_mut()
            .unwrap()
    }
    /// Apply a map function.
    ///
    /// This will use the last timeline's state, which is of type `T` to
    /// construct a mapped state of type `E`, then use this mapped state to
    /// create a new [`ItemTimeline<E>`] and push to the end of the timleines.
    ///
    /// If there is a planning static anim, it will get submitted and the new
    /// timeline will start planning a static anim immediately just like the
    /// static anim goes "cross" two timeline of different type.
    pub fn apply_map<T: Extract + Clone + 'static, E: Extract + Clone + 'static>(
        &mut self,
        map_fn: impl FnOnce(T) -> E,
    ) {
        let (state, end_sec, is_showing) = {
            let timeline = self.get_mut::<T>();
            let is_showing = timeline.planning_static_start_sec.is_some();
            timeline.seal();
            (
                timeline.snapshot().clone(),
                timeline.end_sec().unwrap_or(timeline.cur_sec()),
                is_showing,
            )
        };
        let new_state = map_fn(state);
        let mut new_timeline = ItemTimeline::new(new_state);
        new_timeline.forward_to(end_sec);
        if is_showing {
            new_timeline.show();
        }
        self.inner.push(Box::new(new_timeline));
    }
}

// MARK: ItemTimeline<T>
/// `ItemTimeline<T>` is used to encode animations for a single type `T`,
/// it contains a list of [`AnimationSpan<T>`] and the corresponding metadata for each span.
pub struct ItemTimeline<T> {
    type_name: String,
    anims: Vec<(AnimationSpan<T>, std::ops::Range<f64>)>,

    // Followings are states use while constructing
    cur_sec: f64,
    /// The state used for static anim.
    state: T,
    /// The start time of the planning static anim.
    /// When it is true, it means that it is showing.
    planning_static_start_sec: Option<f64>,
}
// ANCHOR_END: ItemTimeline

impl<T: 'static> ItemTimeline<T> {
    /// Create a new timeline with the initial state
    ///
    /// The timeline is hidden by default, because we don't know when the first anim starts.
    /// And this allow us to use [`ItemTimeline::forward`] and [`ItemTimeline::forward_to`]
    /// to adjust the start time of the first anim.
    pub fn new(state: T) -> Self {
        Self {
            type_name: std::any::type_name::<T>().to_string(),
            anims: vec![],
            // extractor: None,
            cur_sec: 0.0,
            state,
            planning_static_start_sec: None,
        }
    }
}

/// Info of an animation
pub struct AnimationInfo {
    /// The name of the animation
    pub anim_name: String,
    /// The time range of the animation
    pub range: std::ops::Range<f64>,
}

impl<T: Extract + Any + Clone + 'static> TimelineFunc for ItemTimeline<T> {
    fn start_sec(&self) -> Option<f64> {
        self.start_sec()
    }
    fn end_sec(&self) -> Option<f64> {
        self.end_sec()
    }
    fn seal(&mut self) {
        // println!("seal");
        self._submit_planning_static_anim();
    }
    fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    /// The [`ItemTimeline::state`] should be `Some`
    fn show(&mut self) {
        self.show();
    }
    fn hide(&mut self) {
        self.hide();
    }
    fn forward(&mut self, duration_secs: f64) {
        self.forward(duration_secs);
    }
    fn get_animation_infos(&self) -> Vec<AnimationInfo> {
        // const MAX_INFO_CNT: usize = 100;
        self.anims
            .iter()
            .map(|(anim, range)| AnimationInfo {
                anim_name: anim.type_name().to_string(),
                range: range.clone(),
            })
            // .take(MAX_INFO_CNT)
            .collect()
    }
    fn type_name(&self) -> &str {
        &self.type_name
    }
    // fn eval_sec_any(&self, target_sec: f64) -> Option<(EvalResult<dyn Any>, usize)> {
    //     self.eval_sec(target_sec)
    //         .map(|(res, idx)| (res.into_any(), idx))
    // }
    fn eval_primitives_at_sec(&self, target_sec: f64) -> Option<(EvalResult<Primitives>, u64)> {
        self.eval_at_sec(target_sec)
            .map(|(res, idx)| (res.map(|res| res.extract_to_primitives()), idx))
    }
}

impl<T: Clone + 'static> ItemTimeline<T> {
    /// Get the start sec
    pub fn start_sec(&self) -> Option<f64> {
        self.anims.first().map(|(_, range)| range.start)
    }
    /// Get the end sec
    pub fn end_sec(&self) -> Option<f64> {
        self.anims.last().map(|(_, range)| range.end)
    }
    /// Get the current second
    pub fn cur_sec(&self) -> f64 {
        self.cur_sec
    }
    /// Get the reference of current item state
    pub fn snapshot_ref(&self) -> &T {
        &self.state
    }
    /// Get the current item state
    pub fn snapshot(&self) -> T {
        self.state.clone()
    }
    /// Do something on the timeline with current snapshot captured
    pub fn with_snapshot<R>(&mut self, f: impl Fn(&mut Self, T) -> R) -> R {
        let state = self.snapshot();
        f(self, state)
    }
    /// Update the state
    pub fn update(&mut self, state: T) -> &mut Self {
        self.update_with(|s| *s = state)
    }
    /// Update the state with `update_func`
    pub fn update_with(&mut self, update_func: impl FnOnce(&mut T)) -> &mut Self {
        let showing = self._submit_planning_static_anim();
        update_func(&mut self.state);
        if showing {
            self.show();
        }
        self
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
        if let Some(start) = self.planning_static_start_sec.take() {
            self.anims.push((
                AnimationSpan::from_evaluator(Evaluator::Static(Arc::new(self.state.clone()))),
                start..self.cur_sec,
            ));
            return true;
        }
        false
    }
    /// Plays an anim with `anim_func`.
    pub fn play_with(&mut self, anim_func: impl FnOnce(T) -> AnimationSpan<T>) -> &mut Self {
        self.play(anim_func(self.state.clone()))
    }
    /// Plays an anim.
    pub fn play(&mut self, anim: AnimationSpan<T>) -> &mut Self {
        self._submit_planning_static_anim();
        let res = anim.eval_alpha(1.0).into_owned();
        let duration = anim.duration_secs;
        let end = self.cur_sec + duration;
        self.anims.push((anim, self.cur_sec..end));
        self.cur_sec = end;
        self.update(res);
        self.show();
        self
    }
    /// Evaluate the state at `alpha`
    pub fn eval_at_alpha(&self, alpha: f64) -> Option<(EvalResult<T>, u64)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };
        self.eval_at_sec(alpha * (end - start) + start)
    }
    /// Evaluate the state at `target_sec`
    pub fn eval_at_sec(&self, target_sec: f64) -> Option<(EvalResult<T>, u64)> {
        let (Some(start), Some(end)) = (self.start_sec(), self.end_sec()) else {
            return None;
        };

        if !(start..=end).contains(&target_sec) {
            return None;
        }

        self.anims
            .iter()
            .enumerate()
            .find_map(|(idx, (anim, range))| {
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
                (anim.eval_alpha(alpha), idx as u64)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{primitives::vitem::VItemPrimitive, timeline::ItemTimeline};

    #[test]
    fn test_item_timeline() {
        let vitem = VItemPrimitive {
            points2d: vec![],
            fill_rgbas: vec![],
            stroke_rgbas: vec![],
            stroke_widths: vec![],
        };
        let mut timeline = ItemTimeline::new(vitem.clone());
        assert!(timeline.eval_at_alpha(0.0).is_none());
        timeline.show();
        timeline.forward(1.0);
        timeline.seal();

        assert_eq!(timeline.start_sec(), Some(0.0));
        assert_eq!(timeline.end_sec(), Some(1.0));

        let (res, _) = timeline.eval_at_alpha(0.0).unwrap();
        assert_eq!(res.as_ref(), &vitem);
        let (res, _) = timeline.eval_at_alpha(0.5).unwrap();
        assert_eq!(res.as_ref(), &vitem);
        let (res, _) = timeline.eval_at_alpha(1.0).unwrap();
        assert_eq!(res.as_ref(), &vitem);
    }

    #[test]
    fn test_item_dyn_timelines() {
        let vitem = VItemPrimitive {
            points2d: vec![],
            fill_rgbas: vec![],
            stroke_rgbas: vec![],
            stroke_widths: vec![],
        };
        let mut timeline = ItemDynTimelines::new();
        timeline.push(ItemTimeline::new(vitem.clone()));
        assert!(timeline.eval_primitives_at_alpha(0.0).is_none());

        timeline.get_dyn_mut().show();
        timeline.get_dyn_mut().forward(1.0);
        timeline.get_dyn_mut().seal();

        assert_eq!(timeline.get_dyn().start_sec(), Some(0.0));
        assert_eq!(timeline.get_dyn().end_sec(), Some(1.0));

        let (res, _) = timeline.eval_primitives_at_alpha(0.0).unwrap();
        assert_eq!(res.as_ref(), &Primitives::VItemPrimitive(vec![vitem]));
    }
}
