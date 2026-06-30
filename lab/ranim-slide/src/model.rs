use egui::{Color32, Pos2, Rect, Vec2, pos2, vec2};

pub const SLIDE_WIDTH: f32 = 1280.0;
pub const SLIDE_HEIGHT: f32 = 720.0;
pub const SLIDE_SIZE: Vec2 = vec2(SLIDE_WIDTH, SLIDE_HEIGHT);

#[derive(Debug, Clone)]
pub struct Deck {
    pub pages: Vec<SlidePage>,
    pub selected_page: usize,
    next_id: u64,
}

impl Default for Deck {
    fn default() -> Self {
        let mut deck = Self {
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
            elements: Vec::new(),
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

    pub fn add_rect_to_current_page(&mut self) {
        let id = self.alloc_id();
        let element = Element {
            id,
            name: format!("Rect {id}"),
            kind: ElementKind::Rect,
            rect: Rect::from_min_size(pos2(440.0, 250.0), vec2(320.0, 180.0)),
            fill: Color32::from_rgb(68, 119, 245),
            stroke: Color32::from_rgb(245, 247, 250),
            selected: false,
        };
        self.current_page_mut().insert_element(element);
    }

    pub fn add_text_to_current_page(&mut self) {
        let id = self.alloc_id();
        let element = Element {
            id,
            name: format!("Text {id}"),
            kind: ElementKind::Text {
                content: "Text".to_owned(),
                size: 44.0,
            },
            rect: Rect::from_min_size(pos2(460.0, 310.0), vec2(360.0, 72.0)),
            fill: Color32::from_rgb(30, 35, 42),
            stroke: Color32::TRANSPARENT,
            selected: false,
        };
        self.current_page_mut().insert_element(element);
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
    pub elements: Vec<Element>,
    pub selected_element: Option<u64>,
}

impl SlidePage {
    pub fn insert_element(&mut self, element: Element) {
        let id = element.id;
        self.elements.push(element);
        self.select_element(Some(id));
    }

    pub fn select_element(&mut self, id: Option<u64>) {
        self.selected_element = id;
        for element in &mut self.elements {
            element.selected = Some(element.id) == id;
        }
    }

    pub fn selected_element_mut(&mut self) -> Option<&mut Element> {
        let selected_id = self.selected_element?;
        self.elements
            .iter_mut()
            .find(|element| element.id == selected_id)
    }

    pub fn delete_selected(&mut self) {
        let Some(selected_id) = self.selected_element else {
            return;
        };

        self.elements.retain(|element| element.id != selected_id);
        self.selected_element = None;
    }

    pub fn element_at(&self, slide_pos: Pos2) -> Option<u64> {
        self.elements
            .iter()
            .rev()
            .find(|element| element.rect.contains(slide_pos))
            .map(|element| element.id)
    }
}

#[derive(Debug, Clone)]
pub struct Element {
    pub id: u64,
    pub name: String,
    pub kind: ElementKind,
    pub rect: Rect,
    pub fill: Color32,
    pub stroke: Color32,
    pub selected: bool,
}

impl Element {
    pub fn translate(&mut self, delta: Vec2) {
        self.rect = self.rect.translate(delta);
    }

    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.rect = Rect::from_min_size(pos2(x, y), self.rect.size());
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.rect = Rect::from_min_size(self.rect.min, vec2(width.max(1.0), height.max(1.0)));
    }
}

#[derive(Debug, Clone)]
pub enum ElementKind {
    Rect,
    Text { content: String, size: f32 },
}
