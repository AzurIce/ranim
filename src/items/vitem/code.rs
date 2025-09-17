use diff_match_patch_rs::{Efficient, Ops};

use crate::{
    items::{
        Group,
        vitem::{VItem, svg::SvgItem, typst::typst_svg},
    },
    render::primitives::{Extract, vitem::VItemPrimitive},
    traits::{
        Alignable, BoundingBox, FillColor, Interpolatable, Opacity, Rotate, Scale, Shift,
        StrokeColor, StrokeWidth, With,
    },
};

/// A code block
#[derive(Clone)]
pub struct Code {
    chars: String,
    vitems: Vec<VItem>,
}

impl Code {
    /// Create an inline code block
    pub fn new_inline(code: &str) -> Self {
        let svg = SvgItem::new(typst_svg(&format!("`{code}`")));
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = Vec::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { chars, vitems }
    }
    /// Create a multiline code block
    pub fn new_multiline(code: &str, language: Option<&str>) -> Self {
        let language = language.unwrap_or("");
        let svg = SvgItem::new(typst_svg(&format!("```{language}\n{code}\n```")));
        let chars = code
            .replace(" ", "")
            .replace("\n", "")
            .replace("\r", "")
            .replace("\t", "");

        let vitems = Vec::<VItem>::from(svg);
        assert_eq!(chars.len(), vitems.len());
        Self { vitems, chars }
    }
}

impl Alignable for Code {
    fn is_aligned(&self, other: &Self) -> bool {
        self.vitems.len() == other.vitems.len()
            && self
                .vitems
                .iter()
                .zip(&other.vitems)
                .all(|(a, b)| a.is_aligned(b))
    }
    fn align_with(&mut self, other: &mut Self) {
        let dmp = diff_match_patch_rs::DiffMatchPatch::new();
        let diffs = dmp
            .diff_main::<Efficient>(&self.chars, &other.chars)
            .unwrap();

        let len = self.vitems.len().max(other.vitems.len());
        let mut vitems_self: Vec<VItem> = Vec::with_capacity(len);
        let mut vitems_other: Vec<VItem> = Vec::with_capacity(len);
        let mut ia = 0;
        let mut ib = 0;
        let mut last_neq_idx_a = 0;
        let mut last_neq_idx_b = 0;
        let align_and_push_diff = |vitems_self: &mut Vec<VItem>,
                                   vitems_other: &mut Vec<VItem>,
                                   ia,
                                   ib,
                                   last_neq_idx_a,
                                   last_neq_idx_b| {
            if last_neq_idx_a != ia || last_neq_idx_b != ib {
                let mut vitems_a = self.vitems[last_neq_idx_a..ia]
                    .iter()
                    .cloned()
                    .collect::<Group<_>>();
                let mut vitems_b = other.vitems[last_neq_idx_b..ib]
                    .iter()
                    .cloned()
                    .collect::<Group<_>>();
                if vitems_a.is_empty() {
                    vitems_a.extend(vitems_b.iter().map(|x| {
                        x.clone().with(|x| {
                            x.shrink();
                        })
                    }));
                }
                if vitems_b.is_empty() {
                    vitems_b.extend(vitems_a.iter().map(|x| {
                        x.clone().with(|x| {
                            x.shrink();
                        })
                    }));
                }
                if last_neq_idx_a != ia && last_neq_idx_b != ib {
                    vitems_a.align_with(&mut vitems_b);
                }
                vitems_self.extend(vitems_a);
                vitems_other.extend(vitems_b);
            }
        };

        for diff in &diffs {
            // println!("[{ia}] {last_neq_idx_a} [{ib}] {last_neq_idx_b}");
            // println!("{diff:?}");
            match diff.op() {
                Ops::Equal => {
                    align_and_push_diff(
                        &mut vitems_self,
                        &mut vitems_other,
                        ia,
                        ib,
                        last_neq_idx_a,
                        last_neq_idx_b,
                    );
                    let l = diff.size();
                    vitems_self.extend(self.vitems[ia..ia + l].iter().cloned());
                    vitems_other.extend(other.vitems[ib..ib + l].iter().cloned());
                    ia += l;
                    ib += l;
                    last_neq_idx_a = ia;
                    last_neq_idx_b = ib;
                }
                Ops::Delete => {
                    ia += diff.size();
                }
                Ops::Insert => {
                    ib += diff.size();
                }
            }
        }
        align_and_push_diff(
            &mut vitems_self,
            &mut vitems_other,
            ia,
            ib,
            last_neq_idx_a,
            last_neq_idx_b,
        );

        assert_eq!(vitems_self.len(), vitems_other.len());
        vitems_self
            .iter_mut()
            .zip(vitems_other.iter_mut())
            .for_each(|(a, b)| {
                // println!("{i} {}", a.is_aligned(b));
                // println!("{} {}", a.vpoints.len(), b.vpoints.len());
                if !a.is_aligned(b) {
                    a.align_with(b);
                }
            });

        self.vitems = vitems_self;
        other.vitems = vitems_other;
    }
}

impl Interpolatable for Code {
    fn lerp(&self, target: &Self, t: f64) -> Self {
        let vitems = self
            .vitems
            .iter()
            .zip(&target.vitems)
            .map(|(a, b)| a.lerp(b, t))
            .collect::<Vec<_>>();
        Self {
            chars: self.chars.clone(),
            vitems,
        }
    }
}

impl Extract for Code {
    type Target = Vec<VItemPrimitive>;
    fn extract(&self) -> Self::Target {
        self.vitems.iter().map(Extract::extract).collect()
    }
}

impl BoundingBox for Code {
    fn get_bounding_box(&self) -> [glam::DVec3; 3] {
        self.vitems.get_bounding_box()
    }
}

impl Shift for Code {
    fn shift(&mut self, shift: glam::DVec3) -> &mut Self {
        self.vitems.shift(shift);
        self
    }
}

impl Rotate for Code {
    fn rotate_by_anchor(
        &mut self,
        angle: f64,
        axis: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.vitems.rotate_by_anchor(angle, axis, anchor);
        self
    }
}

impl Scale for Code {
    fn scale_by_anchor(
        &mut self,
        scale: glam::DVec3,
        anchor: crate::components::Anchor,
    ) -> &mut Self {
        self.vitems.scale_by_anchor(scale, anchor);
        self
    }
}

impl FillColor for Code {
    fn fill_color(&self) -> color::AlphaColor<color::Srgb> {
        self.vitems[0].fill_color()
    }
    fn set_fill_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.vitems.set_fill_color(color);
        self
    }
    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_fill_opacity(opacity);
        self
    }
}

impl StrokeColor for Code {
    fn stroke_color(&self) -> color::AlphaColor<color::Srgb> {
        self.vitems[0].fill_color()
    }
    fn set_stroke_color(&mut self, color: color::AlphaColor<color::Srgb>) -> &mut Self {
        self.vitems.set_stroke_color(color);
        self
    }
    fn set_stroke_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_stroke_opacity(opacity);
        self
    }
}

impl Opacity for Code {
    fn set_opacity(&mut self, opacity: f32) -> &mut Self {
        self.vitems.set_fill_opacity(opacity);
        self.vitems.set_stroke_opacity(opacity);
        self
    }
}

impl StrokeWidth for Code {
    fn apply_stroke_func(
        &mut self,
        f: impl for<'a> Fn(&'a mut [crate::components::width::Width]),
    ) -> &mut Self {
        self.vitems.iter_mut().for_each(|vitem| {
            vitem.apply_stroke_func(&f);
        });
        self
    }
    fn set_stroke_width(&mut self, width: f32) -> &mut Self {
        self.vitems.set_stroke_width(width);
        self
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn foo() {
        let code_a = r#"#include <iostream>
using namespace std;

int main() {
    cout << "Hello World!" << endl;
}
"#;
        let mut code_a = Code::new_multiline(code_a, Some("cpp"));
        let code_b = r#"fn main() {
    println!("Hello World!");
}"#;
        let mut code_b = Code::new_multiline(code_b, Some("rust"));

        code_a.align_with(&mut code_b);
    }
}
