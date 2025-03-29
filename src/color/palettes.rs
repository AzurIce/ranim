pub use color::palette::css;

pub mod manim {
    use color::{AlphaColor, Srgb};

    use crate::color::rgb;

    /// <div style="background-color: #1C758A; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_E: AlphaColor<Srgb> = rgb(0.11, 0.46, 0.54);
    /// <div style="background-color: #29ABCA; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_D: AlphaColor<Srgb> = rgb(0.16, 0.67, 0.79);
    /// <div style="background-color: #58C4DD; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_C: AlphaColor<Srgb> = rgb(0.35, 0.77, 0.87);
    /// <div style="background-color: #9CDCEB; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_B: AlphaColor<Srgb> = rgb(0.61, 0.86, 0.92);
    /// <div style="background-color: #C7E9F1; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLUE_A: AlphaColor<Srgb> = rgb(0.78, 0.91, 0.95);

    /// <div style="background-color: #49A88F; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL_E: AlphaColor<Srgb> = rgb(0.29, 0.66, 0.56);
    /// <div style="background-color: #55C1A7; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL_D: AlphaColor<Srgb> = rgb(0.33, 0.76, 0.66);
    /// <div style="background-color: #5CD0B3; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL_C: AlphaColor<Srgb> = rgb(0.36, 0.82, 0.70);
    /// <div style="background-color: #76DDC0; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL_B: AlphaColor<Srgb> = rgb(0.46, 0.87, 0.75);
    /// <div style="background-color: #ACEAD7; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const TEAL_A: AlphaColor<Srgb> = rgb(0.68, 0.92, 0.84);

    /// <div style="background-color: #699C52; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_E: AlphaColor<Srgb> = rgb(0.41, 0.61, 0.32);
    /// <div style="background-color: #77B05D; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_D: AlphaColor<Srgb> = rgb(0.47, 0.69, 0.37);
    /// <div style="background-color: #83C167; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_C: AlphaColor<Srgb> = rgb(0.51, 0.76, 0.40);
    /// <div style="background-color: #A6CF8C; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_B: AlphaColor<Srgb> = rgb(0.65, 0.81, 0.55);
    /// <div style="background-color: #C9E2AE; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_A: AlphaColor<Srgb> = rgb(0.79, 0.89, 0.68);

    /// <div style="background-color: #E8C11C; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_E: AlphaColor<Srgb> = rgb(0.91, 0.76, 0.11);
    /// <div style="background-color: #F4D345; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_D: AlphaColor<Srgb> = rgb(0.96, 0.83, 0.27);
    /// <div style="background-color: #FFFF00; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_C: AlphaColor<Srgb> = rgb(1.00, 1.00, 0.00);
    /// <div style="background-color: #FFEA94; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_B: AlphaColor<Srgb> = rgb(1.00, 0.92, 0.58);
    /// <div style="background-color: #FFF1B6; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const YELLOW_A: AlphaColor<Srgb> = rgb(1.00, 0.95, 0.71);

    /// <div style="background-color: #C78D46; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD_E: AlphaColor<Srgb> = rgb(0.78, 0.55, 0.28);
    /// <div style="background-color: #E1A158; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD_D: AlphaColor<Srgb> = rgb(0.88, 0.63, 0.35);
    /// <div style="background-color: #F0AC5F; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD_C: AlphaColor<Srgb> = rgb(0.94, 0.68, 0.37);
    /// <div style="background-color: #F9B775; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD_B: AlphaColor<Srgb> = rgb(0.98, 0.72, 0.46);
    /// <div style="background-color: #F7C797; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GOLD_A: AlphaColor<Srgb> = rgb(0.97, 0.78, 0.59);

    /// <div style="background-color: #CF5044; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED_E: AlphaColor<Srgb> = rgb(0.81, 0.31, 0.27);
    /// <div style="background-color: #E65A4C; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED_D: AlphaColor<Srgb> = rgb(0.90, 0.35, 0.30);
    /// <div style="background-color: #FC6255; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED_C: AlphaColor<Srgb> = rgb(0.99, 0.38, 0.33);
    /// <div style="background-color: #FF8080; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED_B: AlphaColor<Srgb> = rgb(1.00, 0.50, 0.50);
    /// <div style="background-color: #F7A1A3; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const RED_A: AlphaColor<Srgb> = rgb(0.97, 0.63, 0.64);

    /// <div style="background-color: #94424F; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON_E: AlphaColor<Srgb> = rgb(0.58, 0.26, 0.31);
    /// <div style="background-color: #A24D61; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON_D: AlphaColor<Srgb> = rgb(0.64, 0.30, 0.38);
    /// <div style="background-color: #C55F73; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON_C: AlphaColor<Srgb> = rgb(0.77, 0.37, 0.45);
    /// <div style="background-color: #EC92AB; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON_B: AlphaColor<Srgb> = rgb(0.92, 0.57, 0.67);
    /// <div style="background-color: #ECABC1; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const MAROON_A: AlphaColor<Srgb> = rgb(0.93, 0.67, 0.76);

    /// <div style="background-color: #644172; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE_E: AlphaColor<Srgb> = rgb(0.39, 0.26, 0.45);
    /// <div style="background-color: #715582; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE_D: AlphaColor<Srgb> = rgb(0.44, 0.33, 0.51);
    /// <div style="background-color: #9A72AC; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE_C: AlphaColor<Srgb> = rgb(0.60, 0.45, 0.68);
    /// <div style="background-color: #B189C6; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE_B: AlphaColor<Srgb> = rgb(0.69, 0.54, 0.78);
    /// <div style="background-color: #CAA3E8; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PURPLE_A: AlphaColor<Srgb> = rgb(0.79, 0.64, 0.91);

    /// <div style="background-color: #222222; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_E: AlphaColor<Srgb> = rgb(0.13, 0.13, 0.13);
    /// <div style="background-color: #444444; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_D: AlphaColor<Srgb> = rgb(0.27, 0.27, 0.27);
    /// <div style="background-color: #888888; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_C: AlphaColor<Srgb> = rgb(0.53, 0.53, 0.53);
    /// <div style="background-color: #BBBBBB; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_B: AlphaColor<Srgb> = rgb(0.73, 0.73, 0.73);
    /// <div style="background-color: #DDDDDD; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_A: AlphaColor<Srgb> = rgb(0.87, 0.87, 0.87);

    /// <div style="background-color: #FFFFFF; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const WHITE: AlphaColor<Srgb> = rgb(1.00, 1.00, 1.00);
    /// <div style="background-color: #000000; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const BLACK: AlphaColor<Srgb> = rgb(0.00, 0.00, 0.00);
    /// <div style="background-color: #00FF00; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREEN_SCREEN: AlphaColor<Srgb> = rgb(0.00, 1.00, 0.00);

    /// <div style="background-color: #736357; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const GREY_BROWN: AlphaColor<Srgb> = rgb(0.45, 0.39, 0.34);
    /// <div style="background-color: #CD853F; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_BROWN: AlphaColor<Srgb> = rgb(0.80, 0.52, 0.25);

    /// <div style="background-color: #D147BD; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const PINK: AlphaColor<Srgb> = rgb(0.82, 0.28, 0.74);
    /// <div style="background-color: #DC75CD; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const LIGHT_PINK: AlphaColor<Srgb> = rgb(0.86, 0.46, 0.80);

    /// <div style="background-color: #FF862F; width: 10px; padding: 10px; border: 1px solid;"></div>
    pub const ORANGE: AlphaColor<Srgb> = rgb(1.00, 0.53, 0.18);
}
