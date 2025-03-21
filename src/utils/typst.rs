use std::io::Write;

use regex::bytes::Regex;

pub fn compile_typst_code(typst_code: &str) -> String {
    let mut child = std::process::Command::new("typst")
        .arg("compile")
        .arg("-")
        .arg("-")
        .arg("-fsvg")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("failed to spawn typst");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(typst_code.as_bytes())
            .expect("failed to write to typst's stdin");
    }

    let output = child.wait_with_output().unwrap().stdout;

    let re = Regex::new(r"<path[^>]*(?:>.*?<\/path>|\/>)").unwrap();
    let removed_bg = re.replace(&output, b"");

    // println!("{}", String::from_utf8_lossy(&output));
    // println!("{}", String::from_utf8_lossy(&removed_bg));
    String::from_utf8_lossy(&removed_bg).to_string()
}

#[macro_export]
macro_rules! typst_svg {
    ($typst_code:expr) => {{
        use $crate::utils::typst::compile_typst_code;

        let mut typst_code = r##"
            #set page(margin: 0cm)
            #set text(fill: rgb("#ffffff"))
        "##
        .to_string();
        typst_code.push_str($typst_code);
        // println!("{}", typst_code);
        let svg = compile_typst_code(typst_code.as_str());
        svg
    }};
}

#[macro_export]
macro_rules! typst_tree {
    ($typst_code:expr) => {{
        use $crate::typst_svg;
        usvg::Tree::from_str(&typst_svg!($typst_code), &usvg::Options::default())
            .expect("failed to parse svg")
    }};
}

#[cfg(test)]
mod test {
    #[test]
    fn test_typst() {
        let svg = typst_svg!(r#"#text(font: "LXGW Bright")[软]"#);
        println!("{}", svg);
        // let svg = typst_svg!(
        //     r#"
        //     #text(20pt)[hello]
        // "#
        // );
        // println!("{}", svg);
        // let svg = typst_svg!(
        //     r#"
        //     #text(20pt)[你好]
        // "#
        // );
        // println!("{}", svg);
        // let svg = typst_svg!(
        //     r#"
        //     #text(60pt)[R]
        // "#
        // );
        // println!("{}", svg);
    }
}
