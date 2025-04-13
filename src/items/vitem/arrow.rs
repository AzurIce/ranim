use glam::DVec3;

// use ranim_macros::Item;
use crate::items::{Item, IterMutRabjects, MutParts, Rabject};

use crate::items::Blueprint;
use crate::prelude::RanimTimeline;

use super::{Circle, Line, VItem};

// MARK: ArrowTip
#[derive(Clone, Item)]
#[item(VItem)]
pub struct Tip(VItem);

impl Tip {
    pub fn new() -> Self {
        Self(Circle(1.0).build())
    }
}

/* Generated through derive(Item)
impl Item for Tip {
    type BaseItem = VItem;
    type Rabject<'t> = TipRabject<'t>;
    fn insert_into_timeline<'t>(self, ranim_timeline: &'t RanimTimeline) -> Self::Rabject<'t> {
        TipRabject(Item::insert_into_timeline(self.0, ranim_timeline))
    }
}

pub struct TipRabject<'t>(pub <VItem as Item>::Rabject<'t>);

pub struct TipMutParts<'a>(pub <VItem as MutParts<'a>>::Mut);

impl<'a> MutParts<'a> for Tip {
    type Owned = Tip;
    type Mut = TipMutParts<'a>;
    fn mut_parts(&'a mut self) -> Self::Mut {
        TipMutParts(self.0.mut_parts())
    }
    fn owned(&'a self) -> Self::Owned {
        Tip(self.0.owned())
    }
}

impl<'a, 't> MutParts<'a> for TipRabject<'t> {
    type Owned = Tip;
    type Mut = TipMutParts<'a>;
    fn mut_parts(&'a mut self) -> Self::Mut {
        TipMutParts(self.0.mut_parts())
    }
    fn owned(&'a self) -> Self::Owned {
        Tip(self.0.owned())
    }
}

impl<'t: 'r, 'r> IterMutRabjects<'t, 'r, VItem> for TipRabject<'t> {
    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut Rabject<'t, VItem>>
    where
        'a: 'b,
        't: 'b,
        VItem: 'b,
    {
        self.0.iter_mut()
    }
}
*/

// MARK: Arrow
#[derive(Clone, Item)]
#[item(VItem)]
pub struct Arrow {
    tip: Tip,
    line: VItem,
}

impl Default for Arrow {
    fn default() -> Self {
        Self::new()
    }
}

impl Arrow {
    pub fn new() -> Self {
        Self {
            tip: Tip::new(),
            line: Line(0.2 * DVec3::NEG_Y, 0.2 * DVec3::Y).build(),
        }
    }
}

/* Generated through derive(Item)
impl Item for Arrow {
    type BaseItem = VItem;
    type Rabject<'t> = ArrowRabject<'t>;
    fn insert_into_timeline<'t>(self, ranim_timeline: &'t RanimTimeline) -> Self::Rabject<'t> {
        ArrowRabject {
            tip: Item::insert_into_timeline(self.tip, ranim_timeline),
            line: Item::insert_into_timeline(self.line, ranim_timeline),
        }
    }
}

pub struct ArrowMutParts<'a> {
    pub tip: <Tip as MutParts<'a>>::Mut,
    pub line: <VItem as MutParts<'a>>::Mut,
}

pub struct ArrowRabject<'t> {
    pub tip: <Tip as Item>::Rabject<'t>,
    pub line: <VItem as Item>::Rabject<'t>,
}

impl<'a> MutParts<'a> for Arrow {
    type Owned = Arrow;
    type Mut = ArrowMutParts<'a>;
    fn mut_parts(&'a mut self) -> Self::Mut {
        ArrowMutParts {
            tip: self.tip.mut_parts(),
            line: self.line.mut_parts(),
        }
    }
    fn owned(&'a self) -> Self::Owned {
        Self::Owned {
            tip: self.tip.owned(),
            line: self.line.owned(),
        }
    }
}

impl<'a, 't> MutParts<'a> for ArrowRabject<'t> {
    type Owned = Arrow;
    type Mut = ArrowMutParts<'a>;
    fn mut_parts(&'a mut self) -> Self::Mut {
        ArrowMutParts {
            tip: self.tip.mut_parts(),
            line: self.line.mut_parts(),
        }
    }
    fn owned(&'a self) -> Self::Owned {
        Arrow {
            tip: self.tip.owned(),
            line: self.line.owned(),
        }
    }
}

impl<'t: 'r, 'r> IterMutRabjects<'t, 'r, VItem> for ArrowRabject<'t> {
    fn iter_mut<'a, 'b>(&'a mut self) -> impl Iterator<Item = &'b mut Rabject<'t, VItem>>
    where
        'a: 'b,
        't: 'b,
        VItem: 'b,
    {
        self.tip.iter_mut().chain(self.line.iter_mut())
    }
}
*/

pub trait ArrowMethods<'a>: MutParts<'a, Mut = ArrowMutParts<'a>> {
    fn set_tip(&'a mut self, tip: VItem);
    fn set_line(&'a mut self, line: VItem);
}

impl<'a, T: MutParts<'a, Mut = ArrowMutParts<'a>>> ArrowMethods<'a> for T {
    fn set_tip(&'a mut self, tip: VItem) {
        *self.mut_parts().tip.0 = tip;
    }

    fn set_line(&'a mut self, line: VItem) {
        *self.mut_parts().line = line;
    }
}