use bevy::prelude::*;
use ranim_bevy::RanimVItem;
use ranim_core::{VItem, traits::Interpolatable};

use super::rgba;

#[derive(Clone, Copy, Debug)]
pub enum RateFunc {
    Linear,
    EaseInQuad,
    EaseOutQuad,
}

impl RateFunc {
    fn sample(self, t: f32) -> f32 {
        match self {
            Self::Linear => t,
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
        }
    }
}

#[derive(Clone)]
pub struct VItemAnimState {
    pub item: VItem,
    pub transform: Transform,
}

impl VItemAnimState {
    pub fn new(item: VItem, transform: Transform) -> Self {
        Self { item, transform }
    }

    pub fn set_fill(&mut self, fill: [f32; 4]) {
        self.item.fill_rgbas = vec![rgba(fill)].into();
    }

    fn lerp(&self, target: &Self, t: f32) -> Self {
        Self {
            item: self.item.lerp(&target.item, t as f64),
            transform: Transform {
                translation: self.transform.translation.lerp(target.transform.translation, t),
                rotation: self.transform.rotation.slerp(target.transform.rotation, t),
                scale: self.transform.scale.lerp(target.transform.scale, t),
            },
        }
    }
}

#[derive(Clone)]
struct VItemAnimSpan {
    start: f32,
    duration: f32,
    rate_func: RateFunc,
    from: VItemAnimState,
    to: VItemAnimState,
}

impl VItemAnimSpan {
    fn end(&self) -> f32 {
        self.start + self.duration
    }

    fn sample(&self, time: f32) -> VItemAnimState {
        if self.duration <= f32::EPSILON {
            return self.to.clone();
        }

        let t = ((time - self.start) / self.duration).clamp(0.0, 1.0);
        self.from.lerp(&self.to, self.rate_func.sample(t))
    }
}

#[derive(Component, Clone)]
pub struct VItemTimeline {
    initial: VItemAnimState,
    spans: Vec<VItemAnimSpan>,
}

impl VItemTimeline {
    fn sample(&self, time: f32) -> VItemAnimState {
        let mut state = self.initial.clone();

        for span in &self.spans {
            if time < span.start {
                break;
            }
            if time <= span.end() {
                return span.sample(time);
            }
            state = span.to.clone();
        }

        state
    }
}

pub struct VItemTimelineBuilder {
    initial: VItemAnimState,
    current: VItemAnimState,
    cursor: f32,
    spans: Vec<VItemAnimSpan>,
}

impl VItemTimelineBuilder {
    pub fn new(initial: VItemAnimState) -> Self {
        Self {
            current: initial.clone(),
            initial,
            cursor: 0.0,
            spans: Vec::new(),
        }
    }

    pub fn cursor(&self) -> f32 {
        self.cursor
    }

    pub fn wait_until(&mut self, time: f32) {
        self.cursor = self.cursor.max(time);
    }

    pub fn play(
        &mut self,
        duration: f32,
        rate_func: RateFunc,
        mutate: impl FnOnce(&mut VItemAnimState),
    ) {
        let from = self.current.clone();
        let mut to = from.clone();
        mutate(&mut to);

        self.spans.push(VItemAnimSpan {
            start: self.cursor,
            duration,
            rate_func,
            from,
            to: to.clone(),
        });

        self.current = to;
        self.cursor += duration;
    }

    pub fn finish(self) -> VItemTimeline {
        VItemTimeline {
            initial: self.initial,
            spans: self.spans,
        }
    }
}

pub fn spawn_timeline(commands: &mut Commands, builder: VItemTimelineBuilder) -> Entity {
    let initial = builder.initial.clone();
    commands
        .spawn((
            RanimVItem::new(initial.item),
            initial.transform,
            builder.finish(),
        ))
        .id()
}

pub fn animate_vitem_timelines(
    time: Res<Time>,
    mut query: Query<(&VItemTimeline, &mut RanimVItem, &mut Transform)>,
) {
    let elapsed = time.elapsed_secs();
    for (timeline, mut vitem, mut transform) in &mut query {
        let state = timeline.sample(elapsed);
        vitem.item = state.item;
        *transform = state.transform;
    }
}
