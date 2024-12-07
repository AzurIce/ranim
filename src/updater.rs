use crate::rabject::Rabject;


pub trait Updater<R: Rabject> {
    #[allow(unused)]
    /// Called when the updater is created
    fn on_create(&mut self, rabject: &mut R){}
    /// Return false if the updater is done, then it will be removed from the scene
    fn on_update(&mut self, rabject: &mut R, dt: f32) -> bool;
    #[allow(unused)]
    /// Called when the updater is destroyed
    fn on_destroy(&mut self, rabject: &mut R){}
}

impl<R: Rabject, T: FnMut(&mut R, f32) -> bool> Updater<R> for T {
    fn on_update(&mut self, rabject: &mut R, dt: f32) -> bool {
        self(rabject, dt)
    }
}
