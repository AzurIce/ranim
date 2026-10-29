//! Typed CPU frame data transported from evaluation to extraction.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use bevy_ecs::prelude::Resource;

/// Item identity: a top-level animation source and its evaluated output part.
pub type FrameItemId = (usize, usize);

/// One evaluated item and its order among all items in the frame.
pub struct FrameItem<T> {
    /// Source-local identity of the item.
    pub id: FrameItemId,
    /// Insertion order across every item type in the frame.
    pub order: usize,
    /// Evaluated CPU data consumed by extraction systems.
    pub value: T,
}

/// Type-independent identity of one evaluated frame item.
#[derive(Debug, Clone, Copy)]
pub struct FrameItemMetadata<'a> {
    /// Identity within the scene.
    pub id: FrameItemId,
    /// Insertion order across all item types.
    pub order: usize,
    /// Concrete type of the evaluated CPU value.
    pub type_id: TypeId,
    /// Concrete type name for diagnostics and inspection.
    pub type_name: &'a str,
}

trait ErasedItems: Any + Send + Sync {
    fn clear(&mut self);
    fn len(&self) -> usize;
    fn metadata_at(&self, index: usize) -> FrameItemMetadata<'_>;
}

impl<T: Send + Sync + 'static> ErasedItems for Vec<FrameItem<T>> {
    fn clear(&mut self) {
        Vec::clear(self);
    }

    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn metadata_at(&self, index: usize) -> FrameItemMetadata<'_> {
        let item = &self[index];
        FrameItemMetadata {
            id: item.id,
            order: item.order,
            type_id: TypeId::of::<T>(),
            type_name: std::any::type_name::<T>(),
        }
    }
}

/// An evaluated frame stored in reusable batches by item type.
///
/// Extraction systems read their input with [`Self::items`]. Items retain a
/// global insertion order even when different types share a frame. Evaluator
/// outputs retain their complete types and nested structure.
#[derive(Resource, Default)]
pub struct EvaluatedFrame {
    batches: HashMap<TypeId, Box<dyn ErasedItems>>,
    len: usize,
}

impl EvaluatedFrame {
    /// Create an empty frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the total number of items across all batches.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return whether the frame contains no items.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Remove all items while retaining each batch and its allocated capacity.
    pub fn clear(&mut self) {
        for batch in self.batches.values_mut() {
            batch.clear();
        }
        self.len = 0;
    }

    /// Append an item, assigning the next global insertion order.
    pub fn push<T: Send + Sync + 'static>(&mut self, id: FrameItemId, value: T) {
        let order = self.len;
        self.batch_mut().push(FrameItem { id, order, value });
        self.len += 1;
    }

    /// Replace the whole frame with items of one type, reusing batch storage.
    pub fn update<T: Send + Sync + 'static>(
        &mut self,
        items: impl IntoIterator<Item = (FrameItemId, T)>,
    ) {
        self.clear();
        let batch = self.batch_mut::<T>();
        batch.extend(
            items
                .into_iter()
                .enumerate()
                .map(|(order, (id, value))| FrameItem { id, order, value }),
        );
        self.len = batch.len();
    }

    /// Read all items of one type, or an empty slice if the type is absent.
    pub fn items<T: 'static>(&self) -> &[FrameItem<T>] {
        let Some(batch) = self.batches.get(&TypeId::of::<T>()) else {
            return &[];
        };
        let batch: &dyn Any = batch.as_ref();
        batch
            .downcast_ref::<Vec<FrameItem<T>>>()
            .expect("frame batch type must match its TypeId")
            .as_slice()
    }

    /// Visit every item's metadata without reading its payload.
    ///
    /// Batch iteration is unordered; use [`FrameItemMetadata::order`] when
    /// scene insertion order is needed.
    pub fn item_metadata(&self) -> impl Iterator<Item = FrameItemMetadata<'_>> + '_ {
        self.batches
            .values()
            .flat_map(|batch| (0..batch.len()).map(move |index| batch.metadata_at(index)))
    }

    fn batch_mut<T: Send + Sync + 'static>(&mut self) -> &mut Vec<FrameItem<T>> {
        let batch = self
            .batches
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(Vec::<FrameItem<T>>::new()));
        let batch: &mut dyn Any = batch.as_mut();
        batch
            .downcast_mut::<Vec<FrameItem<T>>>()
            .expect("frame batch type must match its TypeId")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_batches_preserve_global_insertion_order() {
        let mut frame = EvaluatedFrame::new();
        frame.push((0, 0), 10_u32);
        frame.push((1, 0), String::from("middle"));
        frame.push((2, 0), 20_u32);

        assert_eq!(frame.len(), 3);
        assert!(!frame.is_empty());
        let numbers = frame.items::<u32>();
        assert_eq!(numbers.len(), 2);
        assert_eq!(
            (numbers[0].id, numbers[0].order, numbers[0].value),
            ((0, 0), 0, 10)
        );
        assert_eq!(
            (numbers[1].id, numbers[1].order, numbers[1].value),
            ((2, 0), 2, 20)
        );
        let strings = frame.items::<String>();
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].id, (1, 0));
        assert_eq!(strings[0].order, 1);
        assert_eq!(strings[0].value, "middle");
    }

    #[test]
    fn metadata_reports_all_payload_types_without_retained_empty_batches() {
        let mut frame = EvaluatedFrame::new();
        frame.push((0, 0), 10_u32);
        frame.push((0, 1), String::from("text"));
        let mut metadata = frame.item_metadata().collect::<Vec<_>>();
        metadata.sort_by_key(|item| item.order);
        assert_eq!(metadata.len(), 2);
        assert_eq!(metadata[0].id, (0, 0));
        assert_eq!(metadata[0].type_id, TypeId::of::<u32>());
        assert_eq!(metadata[1].id, (0, 1));
        assert_eq!(metadata[1].type_name, std::any::type_name::<String>());
        frame.clear();
        assert_eq!(frame.item_metadata().count(), 0);
        frame.push((1, 0), 20_u32);
        let metadata = frame.item_metadata().collect::<Vec<_>>();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].type_id, TypeId::of::<u32>());
        assert_eq!(metadata[0].type_name, "u32");
    }

    #[test]
    fn update_replaces_every_type_and_restarts_order() {
        let mut frame = EvaluatedFrame::new();
        frame.push((0, 0), 10_u32);
        frame.push((1, 0), String::from("old"));

        frame.update([
            ((2, 0), String::from("new")),
            ((3, 0), String::from("next")),
        ]);

        assert_eq!(frame.len(), 2);
        assert!(frame.items::<u32>().is_empty());
        let strings = frame.items::<String>();
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].id, (2, 0));
        assert_eq!(strings[0].order, 0);
        assert_eq!(strings[0].value, "new");
        assert_eq!(strings[1].id, (3, 0));
        assert_eq!(strings[1].order, 1);
        assert_eq!(strings[1].value, "next");

        frame.update(std::iter::empty::<(FrameItemId, String)>());
        assert!(frame.is_empty());
        assert!(frame.items::<String>().is_empty());
    }

    #[test]
    fn clear_retains_batches_and_their_capacity() {
        fn capacity<T: 'static>(frame: &EvaluatedFrame) -> usize {
            let batch: &dyn Any = frame.batches[&TypeId::of::<T>()].as_ref();
            batch
                .downcast_ref::<Vec<FrameItem<T>>>()
                .unwrap()
                .capacity()
        }

        let mut frame = EvaluatedFrame::new();
        for index in 0..32 {
            frame.push((index, 0), index);
            frame.push((index, 1), index.to_string());
        }
        let number_capacity = capacity::<usize>(&frame);
        let string_capacity = capacity::<String>(&frame);

        frame.clear();

        assert!(frame.is_empty());
        assert_eq!(frame.len(), 0);
        assert_eq!(frame.batches.len(), 2);
        assert!(frame.items::<usize>().is_empty());
        assert!(frame.items::<String>().is_empty());
        assert_eq!(capacity::<usize>(&frame), number_capacity);
        assert_eq!(capacity::<String>(&frame), string_capacity);

        frame.push((0, 0), 99_usize);
        assert_eq!(frame.items::<usize>()[0].order, 0);
        assert_eq!(capacity::<usize>(&frame), number_capacity);
    }

    #[test]
    fn missing_type_has_no_items() {
        let mut frame = EvaluatedFrame::new();
        assert!(frame.items::<String>().is_empty());
        frame.push((0, 0), 10_u32);
        assert!(frame.items::<String>().is_empty());
    }

    #[test]
    fn frame_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EvaluatedFrame>();
    }
}
