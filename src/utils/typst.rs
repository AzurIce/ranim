use std::io::Write;

use regex::bytes::Regex;
use usvg::Options;

pub fn compile_typst_code(typst_code: &str) -> usvg::Tree {
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
    usvg::Tree::from_data(&removed_bg, &Options::default()).expect("failed to parse svg")
}

#[macro_export]
macro_rules! typst {
    ($typst_code:expr) => {{
        use $crate::utils::typst::compile_typst_code;

        let mut typst_code = r##"
            #set page(margin: 0cm)
            #set text(fill: rgb("#ffffff"))
        "##.to_string();
        typst_code.push_str($typst_code);
        println!("{}", typst_code);
        let svg = compile_typst_code(typst_code.as_str());
        svg
    }};
}
