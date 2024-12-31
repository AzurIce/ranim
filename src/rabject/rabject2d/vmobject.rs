use vello::kurbo;

use crate::scene::{canvas::camera::CanvasCamera, Entity};

use super::bez_path::BezPath;

#[derive(Clone)]
pub enum VMobject {
    Group(Vec<VMobject>),
    Path(BezPath),
}

impl VMobject {
    pub fn apply_affine(&mut self, affine: kurbo::Affine) {
        match self {
            VMobject::Group(children) => {
                for child in children {
                    child.apply_affine(affine);
                }
            }
            VMobject::Path(path) => path.apply_affine(affine),
        }
    }
    
    pub fn print_tree(&self, indent: usize) {
        let indent_str = (0..indent).map(|_| "  ").collect::<String>();
        match self {
            VMobject::Group(children) => {
                println!("{}Group(", indent_str);
                for child in children {
                    child.print_tree(indent + 1);
                }
                println!("{})", indent_str);
            }
            VMobject::Path(path) => {
                println!("{}Path({:?})", indent_str, path);
            }
        }
    }
    
}

impl Entity for VMobject {
    type Renderer = CanvasCamera;
    fn tick(&mut self, _dt: f32) {}
    fn extract(&mut self) {}
    fn prepare(&mut self, _ctx: &crate::context::RanimContext) {}
    fn render(&mut self, _ctx: &mut crate::context::RanimContext, renderer: &mut Self::Renderer) {
        match self {
            VMobject::Group(children) => {
                for child in children {
                    child.render(_ctx, renderer);
                }
            }
            VMobject::Path(node) => node.render(_ctx, renderer),
        }
    }
}
