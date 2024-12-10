use usvg::{Node, Path, Tree};

use super::vmobject::VMobject;

pub enum SvgNode {
    Path(VMobject),
    Group(Vec<SvgNode>),
}

impl SvgNode {
    pub fn path(path: &Path) -> Self {

        for segment in path.data().segments() {
        }

        let path = unimplemented!();        

        Self::Path(path)
    }
}

pub struct SvgMobject {
    svg_string: String,
    root: SvgNode,
}

impl SvgMobject {
    // pub fn new(svg_string: String) -> Self {
    //     Self { svg_string }
    // }
    pub fn from_tree(tree: Tree) -> Self {
        let children = tree
            .root()
            .children()
            .iter()
            .map(|child| match child {
                Node::Group(group) => {
                    unimplemented!()
                }
                Node::Path(path) => SvgNode::path(path),
                Node::Image(image) => {
                    unimplemented!()
                }
                Node::Text(text) => {
                    unimplemented!()
                }
            })
            .collect();
        Self {
            svg_string: "".to_string(),
            root: SvgNode::Group(children),
        }
    }
}

#[cfg(test)]
mod test {
    use usvg::{tiny_skia_path::{self, Stroke}, Options, Tree};

    #[test]
    fn foo() {
        let mut path = tiny_skia_path::PathBuilder::new();
        path.cubic_to(10.0, 10.0, 20.0, 10.0, 30.0, 0.0);
        let path = path.finish().unwrap();

        for segment in path.segments() {
            println!("{:?}", segment);
        }
        println!("{:?}", path.points());
        let stroke = path.stroke(&Stroke::default(), 1.0).unwrap();
        for segment in stroke.segments() {
            println!("{:?}", segment);
        }
        println!("{:?}", stroke.points());

        // let tree = Tree::from_str(TEST_SVG, &Options::default()).unwrap();
        // println!("{:?}", tree.root().children());
    }

    const TEST_SVG: &str = include_str!("../../assets/test.svg");
}
