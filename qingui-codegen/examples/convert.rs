//! 一次性生成 images.rs:cargo run -p qingui-codegen --example convert -- <assets_dir> <out_dir>
fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: convert <assets_dir> <out_dir>");
        std::process::exit(2);
    }
    qingui_codegen::convert(&args[1], &args[2]).unwrap();
}
