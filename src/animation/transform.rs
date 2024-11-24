use std::ops::DerefMut;

use crate::{rabject::{vmobject::VMobject, Interpolatable, RabjectWithId}, scene::Scene};

use super::AnimationFunc;

pub struct Transform {
    target: RabjectWithId<VMobject>,
    source: Option<RabjectWithId<VMobject>>,

    aligned_target: Option<RabjectWithId<VMobject>>,
}

impl Transform {
    pub fn new(target: &RabjectWithId<VMobject>) -> Self {
        Self {
            source: None,
            target: target.clone(),
            aligned_target: None,
        }
    }
}

impl AnimationFunc<VMobject> for Transform {
    fn prepare(&mut self, rabject: &mut RabjectWithId<VMobject>, _: &mut Scene) {
        self.source = Some(rabject.clone());
        self.aligned_target = Some(self.target.clone());
        if !rabject.aligned_with_rabject(&self.aligned_target.as_ref().unwrap()) {
            rabject.align_with_rabject(&mut self.aligned_target.as_mut().unwrap());
        }
    }

    fn interpolate(&mut self, rabject: &mut RabjectWithId<VMobject>, alpha: f32) {
        *(rabject.deref_mut()) = self.source.as_ref().unwrap().lerp(&self.aligned_target.as_ref().unwrap(), alpha);
    }

    fn post_anim(&mut self, rabject: &mut RabjectWithId<VMobject>) {
        *rabject = self.target.clone();
    }
}
