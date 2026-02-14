use std::cell::{Ref, RefCell};

use ranim_core::{
    Extract,
    color::{AlphaColor, Srgb},
    core_item::CoreItem,
    glam::{DAffine3, DMat3, DVec3},
    traits::{
        Aabb, AffineTransform, AffineTransformExt, Discard, FillColor, Locate, Rotate, Scale,
        Shift, With as _,
    },
};

use crate::vitem::{VItem, geometry::anchor::Origin, svg::SvgItem, typst::typst_svg};

/// Simple single-line text item
#[derive(Clone, Debug)]
pub struct TextItem {
    /// Basis
    basis: [DVec3; 2],
    /// Origin
    origin: DVec3,
    /// Text content
    text: String,
    /// Cached items
    items: RefCell<Option<Vec<VItem>>>,
    /// Fill color
    fill_rbgas: AlphaColor<Srgb>,
}

impl Locate<TextItem> for Origin {
    fn locate(&self, target: &TextItem) -> DVec3 {
        target.origin
    }
}

impl TextItem {
    /// Create a new text item
    pub fn new(text: impl Into<String>, em_size: f64) -> Self {
        Self {
            basis: [DVec3::X * em_size, DVec3::Y * em_size],
            origin: DVec3::ZERO,
            text: text.into(),
            items: RefCell::default(),
            fill_rbgas: AlphaColor::WHITE,
        }
    }

    fn generate_items(&self) -> Vec<VItem> {
        let text = self.text.as_str();
        let svg_src = dbg!(typst_svg(
            format!(
                r#"
#set text(
    top-edge: 1em,
    bottom-edge: "baseline",
)
#set page(
    width: auto,
    height: auto,
    margin: 0pt,
    background: rect(width: 100%, height: 100%),
)

{text}
        "#
            )
            .as_str()
        ));
        let items = Vec::<VItem>::from(SvgItem::new(svg_src)).with(|item| {
            let &Self {
                basis: [x_axis, y_axis],
                origin,
                ..
            } = self;
            let [min, max] = item[0].aabb();
            let h = max.y - min.y;
            let mat = DAffine3::from_mat3_translation(
                DMat3::from_cols(x_axis, y_axis, DVec3::ZERO),
                origin,
            );
            item.shift(-min)
                .scale_at_point(DVec3::splat(1. / h), DVec3::ZERO)
                .affine_transform(mat);
        });
        items[1..].into()
    }

    fn items(&self) -> Ref<'_, Vec<VItem>> {
        if self.items.borrow().is_none() {
            let items = self.generate_items();
            self.items.replace(Some(items));
        }
        Ref::map(self.items.borrow(), |v| v.as_ref().unwrap())
    }

    fn transform_items(&self, transformation: impl FnOnce(&mut Vec<VItem>)) {
        if let Some(v) = self.items.borrow_mut().as_mut() {
            transformation(v);
        }
    }
}

impl Aabb for TextItem {
    fn aabb(&self) -> [DVec3; 2] {
        self.items().aabb()
    }
}

impl Shift for TextItem {
    fn shift(&mut self, offset: DVec3) -> &mut Self {
        self.origin += offset;
        self.transform_items(|item| item.shift(offset).discard());
        self
    }
}

impl Rotate for TextItem {
    fn rotate_at_point(&mut self, angle: f64, axis: DVec3, point: DVec3) -> &mut Self {
        self.origin.rotate_at_point(angle, axis, point);
        self.basis.rotate_at_point(angle, axis, DVec3::ZERO);
        self.transform_items(|item| item.rotate_at_point(angle, axis, point).discard());
        self
    }
}

impl Scale for TextItem {
    fn scale_at_point(&mut self, scale: DVec3, point: DVec3) -> &mut Self {
        self.origin.scale_at_point(scale, point);
        self.basis.iter_mut().for_each(|v| *v *= scale);
        self.transform_items(|item| item.scale_at_point(scale, point).discard());
        self
    }
}

impl AffineTransform for TextItem {
    fn affine_transform_at_point(&mut self, mat: DAffine3, origin: DVec3) -> &mut Self {
        self.origin.affine_transform_at_point(mat, origin);
        self.basis
            .iter_mut()
            .for_each(|v| *v = mat.transform_vector3(*v));
        self.transform_items(|item| item.affine_transform_at_point(mat, origin).discard());
        self
    }
}

impl FillColor for TextItem {
    fn fill_color(&self) -> AlphaColor<Srgb> {
        self.fill_rbgas
    }

    fn set_fill_color(&mut self, color: AlphaColor<Srgb>) -> &mut Self {
        self.fill_rbgas = color;
        self
    }

    fn set_fill_opacity(&mut self, opacity: f32) -> &mut Self {
        self.fill_rbgas = self.fill_rbgas.with_alpha(opacity);
        self
    }
}

impl From<TextItem> for Vec<VItem> {
    fn from(item: TextItem) -> Self {
        item.items().clone()
    }
}

impl Extract for TextItem {
    type Target = CoreItem;

    fn extract_into(&self, buf: &mut Vec<Self::Target>) {
        self.items().extract_into(buf);
    }
}
