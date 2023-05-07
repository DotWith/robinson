mod css;
mod dom;
mod html;
mod layout;
mod style;
mod painting;
mod error;

use std::fs;
use std::path::PathBuf;

use clap::Parser;
use error::Result;

/// A toy web rendering engine
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// HTML document
    #[arg(long, default_value = "examples/test.html")]
    html: PathBuf,

    /// CSS stylesheet
    #[arg(long, default_value = "examples/test.css")]
    css: PathBuf,

    /// Output file
    #[arg(long, default_value = "output.png")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read and parse html
    let html = fs::read_to_string(&args.html)?;
    let root_node = html::parse(html);

    // Read and parse css
    let css  = fs::read_to_string(&args.css)?;
    let stylesheet = css::parse(css)?;

    // Since we don't have an actual window, hard-code the "viewport" size.
    let mut viewport: layout::Dimensions = Default::default();
    viewport.content.width  = 800.0;
    viewport.content.height = 600.0;

    // Rendering
    let style_root = style::style_tree(&root_node, &stylesheet);
    let layout_root = layout::layout_tree(&style_root, viewport);

    let canvas = painting::paint(&layout_root, viewport.content);
    let (w, h) = (canvas.width as u32, canvas.height as u32);
    let imgbuf = image::ImageBuffer::from_fn(w, h, move |x, y| {
        let color = canvas.pixels[(y * w + x) as usize];
        image::Rgba([color.r, color.g, color.b, color.a])
    });
    imgbuf.save(&args.output)?;

    Ok(())
}
