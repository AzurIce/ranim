use std::any::Any;

use crate::utils::Id;

use super::{vmobject::VMobject, Rabject, RabjectId};

pub struct VGroup {
    pub(crate) children: Vec<VMobject>,
}

pub struct GroupId(Id);
impl RabjectId for GroupId {
    fn from_id(id: Id) -> Self {
        Self(id)
    }

    fn to_id(&self) -> Id {
        self.0
    }
}

impl Rabject for VGroup {
    type Id = GroupId;
    type Data = ();
    type RenderData = ();
    type RenderResource = ();

    fn insert_to_scene(self, scene: &mut crate::scene::Scene) -> Self::Id {
        let children = self
            .children
            .into_iter()
            .map(|child| child.insert_to_scene(scene))
            .collect::<Vec<_>>();
        GroupId(scene.insert_entity(Entity {
            rabject: (),
            children,
            render_data: None,
            render_resource: None,
        }))
    }
}
