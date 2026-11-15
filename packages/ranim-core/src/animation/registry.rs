//! App-scoped animation type identities for inspection.
//!
//! Identities describe type families, not codecs: registering `Rotate<T>` does
//! not make `T`, closures, or evaluator caches serializable. Generic arguments
//! remain visible in the concrete recipe or definition type name.

use std::collections::BTreeMap;

use bevy_ecs::prelude::Resource;

/// Metadata emitted at an animation type's definition site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationDescriptor {
    /// Defining Cargo package, unaffected by dependency aliases.
    pub package: &'static str,
    /// Version of the defining Cargo package.
    pub version: &'static str,
    /// Definition-site module path (including the Rust crate name).
    pub module_path: &'static str,
    /// Type name, or an explicitly assigned stable name.
    pub name: &'static str,
    /// Rust type family path, without generic arguments.
    pub rust_path: &'static str,
}

impl AnimationDescriptor {
    /// Package-qualified identity, with the Rust crate prefix removed.
    pub fn id(&self) -> String {
        match self.module_path.split_once("::") {
            Some((_, module)) => format!("{}::{module}::{}", self.package, self.name),
            None => format!("{}::{}", self.package, self.name),
        }
    }
}

/// Definition-site identity for a named animation or composition type.
///
/// Derive with `#[derive(ranim::Animation)]`. This does not impose reflection,
/// serialization, or thread-safety requirements on the animation's fields.
pub trait Animation {
    /// Metadata shared by all generic instantiations of this type family.
    const DESCRIPTOR: AnimationDescriptor;
}

/// Animation families explicitly registered by a project's plugins.
#[derive(Resource, Default, Clone)]
pub struct AnimationRegistry {
    types: BTreeMap<String, AnimationDescriptor>,
}

impl AnimationRegistry {
    /// Register a family using any valid concrete instantiation.
    ///
    /// Re-registering exactly the same descriptor is idempotent, allowing
    /// independent plugins to depend on the same family.
    ///
    /// # Panics
    ///
    /// Panics on an identity or Rust-path conflict; registration order never
    /// silently changes which definition an identity resolves to.
    pub fn register<T: Animation>(&mut self) {
        self.register_descriptor(T::DESCRIPTOR);
    }

    /// Register definition-site metadata directly.
    ///
    /// # Panics
    ///
    /// Panics if an existing identity or Rust path has different metadata.
    pub fn register_descriptor(&mut self, descriptor: AnimationDescriptor) {
        let id = descriptor.id();
        if let Some(existing) = self.types.get(&id) {
            assert_eq!(*existing, descriptor, "animation identity conflict: {id}");
            return;
        }
        assert!(
            !self
                .types
                .values()
                .any(|existing| existing.rust_path == descriptor.rust_path),
            "animation Rust path conflict: {}",
            descriptor.rust_path,
        );
        self.types.insert(id, descriptor);
    }

    /// Find a family by its package-qualified identity.
    pub fn get(&self, id: &str) -> Option<&AnimationDescriptor> {
        self.types.get(id)
    }

    /// Resolve a concrete recipe or definition Rust name to its registered family.
    ///
    /// Only the outer type is matched. In particular, a closure adapter does
    /// not inherit the registration of a type mentioned in its arguments.
    pub fn resolve(&self, rust_name: &str) -> Option<&AnimationDescriptor> {
        let family = rust_name.split('<').next().unwrap_or(rust_name);
        self.types
            .values()
            .find(|descriptor| descriptor.rust_path == family)
    }

    /// Registered families in deterministic identity order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &AnimationDescriptor> {
        self.types.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(ranim_macros::Animation)]
    struct Named<T>(T);

    #[test]
    fn derive_uses_definition_package_and_preserves_generic_bounds() {
        let descriptor = Named::<u32>::DESCRIPTOR;
        assert_eq!(descriptor.package, "ranim-core");
        assert_eq!(descriptor.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            descriptor.id(),
            "ranim-core::animation::registry::tests::Named"
        );
        assert_eq!(descriptor, Named::<String>::DESCRIPTOR);
        let mut registry = AnimationRegistry::default();
        registry.register::<Named<u32>>();
        registry.register::<Named<String>>();
        assert_eq!(registry.iter().len(), 1);
        assert_eq!(
            registry.resolve(std::any::type_name::<Named<String>>()),
            Some(&descriptor)
        );
        assert!(registry.resolve("other::Named<u32>").is_none());
        assert!(
            registry
                .resolve("adapter::Pure<ranim_core::animation::registry::tests::Named<u32>>")
                .is_none()
        );
    }

    #[test]
    #[should_panic(expected = "animation identity conflict")]
    fn conflicting_versions_are_rejected() {
        let mut registry = AnimationRegistry::default();
        registry.register::<Named<u32>>();
        registry.register_descriptor(AnimationDescriptor {
            version: "different",
            ..Named::<u32>::DESCRIPTOR
        });
    }
}
