pub trait Updater<T> {
    #[allow(unused)]
    /// Called when the updater is created
    fn on_create(&mut self, rabject: &mut T) {}
    /// Return false if the updater is done, then it will be removed from the scene
    fn on_update(&mut self, rabject: &mut T, dt: f32) -> bool;
    #[allow(unused)]
    /// Called when the updater is destroyed
    fn on_destroy(&mut self, rabject: &mut T) {}
}

impl<T, U: FnMut(&mut T, f32) -> bool> Updater<T> for U {
    fn on_update(&mut self, rabject: &mut T, dt: f32) -> bool {
        self(rabject, dt)
    }
}
