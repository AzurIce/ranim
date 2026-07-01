use egui::{Pos2, Vec2, pos2, vec2};
use ranim_core::{glam::dvec3, prelude::CameraFrame};

use crate::object::{SlideObject, SlideObjectDescriptor};

pub const SLIDE_ASPECT_RATIO: f32 = 16.0 / 9.0;
pub const SLIDE_FRAME_HEIGHT: f32 = 8.0;
pub const MIN_OBJECT_SIZE: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlideFrame {
    pub aspect_ratio: f32,
    pub frame_height: f32,
}

impl Default for SlideFrame {
    fn default() -> Self {
        Self {
            aspect_ratio: SLIDE_ASPECT_RATIO,
            frame_height: SLIDE_FRAME_HEIGHT,
        }
    }
}

impl SlideFrame {
    pub fn width(&self) -> f32 {
        self.frame_height * self.aspect_ratio
    }

    pub fn size(&self) -> Vec2 {
        vec2(self.width(), self.frame_height)
    }

    pub fn min(&self) -> Pos2 {
        pos2(-self.width() / 2.0, -self.frame_height / 2.0)
    }

    pub fn max(&self) -> Pos2 {
        pos2(self.width() / 2.0, self.frame_height / 2.0)
    }

    pub fn camera_frame(&self) -> CameraFrame {
        CameraFrame {
            frame_height: self.frame_height as f64,
            ..CameraFrame::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Deck {
    pub frame: SlideFrame,
    pub pages: Vec<SlidePage>,
    pub selected_page: usize,
    next_id: u64,
}

impl Default for Deck {
    fn default() -> Self {
        let mut deck = Self {
            frame: SlideFrame::default(),
            pages: Vec::new(),
            selected_page: 0,
            next_id: 1,
        };
        deck.push_page();
        deck
    }
}

impl Deck {
    pub fn current_page(&self) -> &SlidePage {
        &self.pages[self.selected_page]
    }

    pub fn current_page_mut(&mut self) -> &mut SlidePage {
        &mut self.pages[self.selected_page]
    }

    pub fn push_page(&mut self) {
        let id = self.alloc_id();
        let idx = self.pages.len() + 1;
        self.pages.push(SlidePage {
            id,
            name: format!("Page {idx}"),
            output_camera: self.frame.camera_frame(),
            elements: Vec::new(),
            selected_camera: false,
            selected_element: None,
        });
        self.selected_page = self.pages.len() - 1;
    }

    pub fn remove_current_page(&mut self) {
        if self.pages.len() <= 1 {
            return;
        }

        self.pages.remove(self.selected_page);
        self.selected_page = self.selected_page.min(self.pages.len() - 1);
    }

    pub fn add_object_to_current_page(&mut self, descriptor: &'static SlideObjectDescriptor) {
        let id = self.alloc_id();
        let element = Element {
            id,
            name: format!("{} {id}", descriptor.display_name),
            z_index: id as i32,
            visible: true,
            locked: false,
            lock_aspect: true,
            selected: false,
            object: (descriptor.create_default)(),
        };
        self.current_page_mut().insert_element(element);
    }

    pub fn duplicate_element_on_current_page(&mut self, id: u64) -> Option<u64> {
        let (mut element, z_index) = {
            let page = self.current_page();
            let element = page.element(id)?.clone();
            (element, page.next_z_index())
        };
        let new_id = self.alloc_id();
        element.id = new_id;
        element.name = format!("{} Copy", element.name);
        element.z_index = z_index;
        element.selected = false;
        element.translate(vec2(0.25, -0.25));
        self.current_page_mut().insert_element(element);
        Some(new_id)
    }

    fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[derive(Debug, Clone)]
pub struct SlidePage {
    pub id: u64,
    pub name: String,
    pub output_camera: CameraFrame,
    pub elements: Vec<Element>,
    pub selected_camera: bool,
    pub selected_element: Option<u64>,
}

impl SlidePage {
    pub fn insert_element(&mut self, element: Element) {
        let id = element.id;
        self.elements.push(element);
        self.select_element(Some(id));
    }

    pub fn select_element(&mut self, id: Option<u64>) {
        self.selected_camera = false;
        self.selected_element = id;
        for element in &mut self.elements {
            element.selected = Some(element.id) == id;
        }
    }

    pub fn select_output_camera(&mut self) {
        self.selected_camera = true;
        self.selected_element = None;
        for element in &mut self.elements {
            element.selected = false;
        }
    }

    pub fn selected_element_mut(&mut self) -> Option<&mut Element> {
        let selected_id = self.selected_element?;
        self.elements
            .iter_mut()
            .find(|element| element.id == selected_id)
    }

    pub fn element(&self, id: u64) -> Option<&Element> {
        self.elements.iter().find(|element| element.id == id)
    }

    #[cfg(test)]
    pub fn element_mut(&mut self, id: u64) -> Option<&mut Element> {
        self.elements.iter_mut().find(|element| element.id == id)
    }

    pub fn delete_selected(&mut self) {
        let Some(selected_id) = self.selected_element else {
            return;
        };

        self.delete_element(selected_id);
    }

    pub fn delete_element(&mut self, id: u64) -> bool {
        let before_len = self.elements.len();
        self.elements.retain(|element| element.id != id);
        if self.selected_element == Some(id) {
            self.selected_element = None;
        }
        before_len != self.elements.len()
    }

    pub fn element_at(&self, scene_pos: Pos2) -> Option<u64> {
        self.elements
            .iter()
            .filter(|element| element.visible && element.object.hit_test(scene_pos))
            .max_by_key(|element| element.z_index)
            .map(|element| element.id)
    }

    fn next_z_index(&self) -> i32 {
        self.elements
            .iter()
            .map(|element| element.z_index)
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub id: u64,
    pub name: String,
    pub z_index: i32,
    pub visible: bool,
    pub locked: bool,
    pub lock_aspect: bool,
    pub selected: bool,
    pub object: Box<dyn SlideObject>,
}

impl Element {
    pub fn translate(&mut self, delta: Vec2) -> bool {
        if self.locked {
            return false;
        }

        self.object.translate(delta);
        true
    }

    pub fn set_pos(&mut self, x: f64, y: f64, z: f64) {
        self.object.set_position3(dvec3(x, y, z));
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.object.set_size(vec2(
            width.max(MIN_OBJECT_SIZE),
            height.max(MIN_OBJECT_SIZE),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::RECTANGLE_DESCRIPTOR;

    #[test]
    fn hidden_elements_are_not_hit_tested() {
        let mut deck = Deck::default();
        deck.add_object_to_current_page(&RECTANGLE_DESCRIPTOR);
        let id = deck.current_page().selected_element.unwrap();
        let center = deck
            .current_page()
            .element(id)
            .unwrap()
            .object
            .bounds()
            .center();

        assert_eq!(deck.current_page().element_at(center), Some(id));

        deck.current_page_mut().element_mut(id).unwrap().visible = false;
        assert_eq!(deck.current_page().element_at(center), None);
    }

    #[test]
    fn duplicated_elements_keep_editor_flags_and_get_new_identity() {
        let mut deck = Deck::default();
        deck.add_object_to_current_page(&RECTANGLE_DESCRIPTOR);
        let id = deck.current_page().selected_element.unwrap();
        {
            let element = deck.current_page_mut().element_mut(id).unwrap();
            element.visible = false;
            element.locked = true;
            element.lock_aspect = false;
        }

        let copy_id = deck.duplicate_element_on_current_page(id).unwrap();
        let copy = deck.current_page().element(copy_id).unwrap();
        assert_ne!(copy_id, id);
        assert_eq!(deck.current_page().selected_element, Some(copy_id));
        assert!(!copy.visible);
        assert!(copy.locked);
        assert!(!copy.lock_aspect);
        assert!(copy.name.ends_with(" Copy"));
    }

    #[test]
    fn selecting_output_camera_clears_element_selection() {
        let mut deck = Deck::default();
        deck.add_object_to_current_page(&RECTANGLE_DESCRIPTOR);
        assert!(deck.current_page().selected_element.is_some());

        deck.current_page_mut().select_output_camera();
        assert!(deck.current_page().selected_camera);
        assert_eq!(deck.current_page().selected_element, None);

        deck.current_page_mut().delete_selected();
        assert_eq!(deck.current_page().elements.len(), 1);
        assert!(deck.current_page().selected_camera);
    }
}
