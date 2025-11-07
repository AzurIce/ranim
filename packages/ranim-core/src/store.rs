use std::{any::Any, cell::RefCell};

use crate::{
    Extract,
    animation::{AnimationCell, CoreItemAnimation},
    core_item::{CoreItem, vitem::VItemPrimitive},
    prelude::CameraFrame,
};

/// A store of animations
///
/// It has interior mutability, because when pushing an animation into it, we
/// need to return a reference to the animation, which is bound to the store's lifetime.
///
/// To allow the mutation, we use a `RefCell<Vec<Box<dyn AnyAnimation>>>` in its inner.
///
/// # Safety Contract
///
/// The following invariants must be maintained:
///
/// - **No mutation after push**: Once an animation is pushed into the store, it should never
///   be mutated or removed. The only allowed mutation is pushing new animations into the store.
///
/// - **No Vec reallocation issues**: The returned references from `push_eval_dynamic` point directly
///   to the heap-allocated `AnimationCell<T>` data inside the `Box<dyn AnyAnimation>`. Even if the
///   `Vec` reallocates (which moves the `Box`es), the heap data itself doesn't move, so the pointers
///   remain valid. This is safe because `Box` owns heap-allocated data, and the data doesn't move
///   when the `Box` is moved within the `Vec`.
pub struct AnimationStore {
    anims: RefCell<Vec<Box<dyn CoreItemAnimation>>>,
}

impl AnimationStore {
    /// Create a new store.
    pub fn new() -> Self {
        Self {
            anims: RefCell::new(Vec::new()),
        }
    }

    /// Push an `AnimationCell<T>` into the store and return a reference to it.
    ///
    /// The returned reference is bound to `&self`'s lifetime, which means it will be invalidated
    /// when the store is dropped. Since we use `RefCell` for interior mutability, we can modify
    /// the internal `Vec` while holding a shared reference `&self`.
    ///
    /// # Safety
    ///
    /// This function uses unsafe code to return a reference that outlives the `RefCell` borrow.
    /// The safety relies on the following guarantees:
    ///
    /// 1. **Pointer validity**: The raw pointer `ptr` points to the heap-allocated `AnimationCell<T>`
    ///    that is now owned by the `Vec<Box<dyn AnyAnimation>>` inside `self.anims`.
    ///
    /// 2. **Memory layout**: When we coerce `Box<AnimationCell<T>>` to `Box<dyn AnyAnimation>`,
    ///    only the vtable pointer changes. The data pointer (pointing to the actual `AnimationCell<T>`
    ///    on the heap) remains the same, so `ptr` is still valid.
    ///
    /// 3. **Vec reallocation safety**: Even if the `Vec` reallocates (which moves the `Box`es),
    ///    the heap-allocated `AnimationCell<T>` data inside each `Box` does not move. The pointer
    ///    `ptr` points directly to this heap data, not to the `Box` itself, so it remains valid
    ///    regardless of `Vec` reallocations. This is a key property of `Box`: moving the `Box`
    ///    doesn't move the data it points to on the heap.
    ///
    /// 4. **Lifetime binding**: The returned reference `&AnimationCell<T>` has a lifetime that is
    ///    inferred from `&self`, ensuring it cannot outlive the store. This is enforced by Rust's
    ///    borrow checker.
    ///
    /// 5. **No mutation after push**: Once pushed, the animation is never mutated or removed,
    ///    so the pointer remains valid for the lifetime of the store.
    pub fn push_animation<T: Any + Extract<Target = CoreItem>>(
        &self,
        anim: AnimationCell<T>,
    ) -> &AnimationCell<T> {
        let boxed = Box::new(anim);

        // Get a raw pointer to the heap-allocated AnimationCell<T> before converting
        let ptr = Box::into_raw(boxed);
        // Reconstruct as Box<AnimationCell<T>>, then coerce to Box<dyn AnyAnimation>
        // This ensures the vtable is properly set up
        let boxed_concrete: Box<AnimationCell<T>> = unsafe { Box::from_raw(ptr) };
        let boxed_trait: Box<dyn CoreItemAnimation> = boxed_concrete;
        self.anims.borrow_mut().push(boxed_trait);
        // SAFETY: See function documentation for detailed safety guarantees.
        // In summary: ptr points to memory owned by the Vec, the Vec won't reallocate
        // until capacity is exceeded (and we're pushing one element), and the returned
        // reference's lifetime is bound to &self, ensuring it cannot outlive the store.
        unsafe { &*ptr }
    }
}

/// A store of [`CoreItem`]s.
#[derive(Default, Clone)]
pub struct CoreItemStore {
    /// Id, CameraFrames
    pub camera_frames: Vec<CameraFrame>,
    /// Id, VItemPrimitive
    pub vitems: Vec<VItemPrimitive>,
}

impl CoreItemStore {
    /// Create an empty store
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the inner store with the given iterator
    pub fn update(&mut self, items: impl Iterator<Item = CoreItem>) {
        self.camera_frames.clear();
        self.vitems.clear();
        for item in items {
            match item {
                CoreItem::CameraFrame(x) => self.camera_frames.push(x),
                CoreItem::VItemPrimitive(x) => self.vitems.push(x),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_animation_store() {
        use crate::animation::Eval;
        use std::marker::PhantomData;
        #[derive(Default)]
        struct A<T: Default> {
            _phantom: PhantomData<T>,
        }
        impl<T: Default> Eval<T> for A<T> {
            fn eval_alpha(&self, _alpha: f64) -> T {
                T::default()
            }
        }

        let store = AnimationStore::new();
        let anim = store.push_animation(A::<VItemPrimitive>::default().into_animation_cell());
        // drop(store); // This should cause a compile error because anim's lifetime is tied to store
        assert_eq!(anim.eval_alpha(0.0), VItemPrimitive::default());
        assert_eq!(
            anim.eval_alpha_core_item(0.0),
            vec![CoreItem::VItemPrimitive(VItemPrimitive::default())]
        );
        drop(store);
    }
}
