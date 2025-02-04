use crate::{items::Entity, prelude::Empty, Rabject};

use super::entity::{EntityAnimation, EntityAnimator};

pub fn wait<T: Entity + 'static>(rabject: Rabject<T>) -> EntityAnimation<T> {
    EntityAnimation::new(rabject.id(), Wait(rabject.data.clone()))
}

pub fn blank<T: Entity + Empty + 'static>(rabject: Rabject<T>) -> EntityAnimation<T> {
    EntityAnimation::new(rabject.id(), Blank)
}

pub struct Wait<T>(T);

impl<T: Entity> EntityAnimator<T> for Wait<T> {
    fn eval_alpha(&mut self, _alpha: f32) -> T {
        self.0.clone()
    }
}

pub struct Blank;

impl<T: Entity + Empty> EntityAnimator<T> for Blank {
    fn eval_alpha(&mut self, _alpha: f32) -> T {
        T::empty()
    }
}
