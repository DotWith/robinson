mod error;

use clap::Parser;
use error::Result;
use robinson_css::StyleSheet;
use robinson_dom::Dom;
use robinson_layout::{Rect, Dimensions};
use robinson_net::Client;

/// A toy web rendering engine
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// HTML document
    #[arg(long, default_value = "examples/test.html")]
    html: String,

    /// CSS stylesheet
    #[arg(long, default_value = "examples/test.css")]
    css: String,

    /// Output file
    #[arg(long, default_value = "output.png")]
    output: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Create the network connection.
    let client = Client::new();

    // Read and parse html
    let html_url = Client::get_url(&args.html)?;
    let html = client.get_to_string(html_url).await?;
    let dom = Dom::parse(&html).unwrap();
    let root_node = dom.children.first().unwrap();

    // Read and parse css
    let css_url = Client::get_url(&args.css)?;
    let css = client.get_to_string(css_url).await?;
    let stylesheet = StyleSheet::parse(&css)?;

    // Since we don't have an actual window, hard-code the "viewport" size.
    let mut viewport = Dimensions {
        content: Rect {
            width: 800.0,
            height: 600.0,
            ..Default::default()
        },
        ..Default::default()
    };

    // Rendering
    let style_root = robinson_style::style_tree(&root_node, &stylesheet);
    let layout_root = robinson_layout::layout_tree(&style_root, &mut viewport);

    let canvas = robinson_paint::paint(&layout_root, viewport.content);
    let (w, h) = (canvas.width as u32, canvas.height as u32);
    let imgbuf = image::ImageBuffer::from_fn(w, h, move |x, y| {
        let color = canvas.pixels[(y * w + x) as usize];
        image::Rgba([color.r, color.g, color.b, color.a])
    });
    imgbuf.save(&args.output)?;

    Ok(())
}
