mod error;

use std::{path::Path, fs};

use clap::Parser;
use error::Result;
use robinson_css::StyleSheet;
use robinson_dom::Dom;
use robinson_layout::{Rect, Dimensions};
use robinson_net::Client;
use robinson_style::StyleTree;

/// A toy web rendering engine
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// HTML document
    #[arg(long, default_value = "examples/test.html")]
    website: String,

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
    let html = client.get_to_string(client.get_url(&args.website)?).await?;
    let dom = Dom::parse(&html).unwrap();
    let root_node = dom.children.first().unwrap();

    let mut stylesheet_links = Vec::new();

    if let Some(root_element) = root_node.element() {
        for ele in root_element.children.iter().filter_map(|e| e.element()) {
            if ele.name == "head" {
                for eee in ele.children.iter().filter_map(|e| e.element()) {
                    if eee.name == "link" {
                        if let Some(_rel) = eee.attributes.get("rel").filter(|&rel| rel == &Some(String::from("stylesheet"))) {
                            if let Some(href) = eee.attributes.get("href").cloned() {
                                let css_url = href.unwrap();
                                let css_path = Path::new(&css_url);
                                let html_path = Path::new(&args.website);
                                let html_url = html_path.parent().unwrap();
                                let connected_path = html_url.join(css_path);
                                stylesheet_links.push(connected_path);
                            }
                        }
                    }
                }
            }
        }
    }

    // Read and parse css
    let mut stylesheets = Vec::new();
    for css in stylesheet_links {
        let css_str = css.to_str().unwrap();
        println!("CSS: {:#?}", css_str);
        let css = client.get_to_string(client.get_url(&css_str)?).await?;
        let stylesheet = StyleSheet::parse(&css)?;
        stylesheets.push(stylesheet);
    }

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
    let style_tree = StyleTree::new(&root_node, &stylesheets);
    let layout_root = robinson_layout::layout_tree(&style_tree.root.borrow(), &mut viewport);

    let canvas = robinson_paint::paint(&layout_root, viewport.content);
    let (w, h) = (canvas.width as u32, canvas.height as u32);
    let imgbuf = image::ImageBuffer::from_fn(w, h, move |x, y| {
        let color = canvas.pixels[(y * w + x) as usize];
        image::Rgba([color.r, color.g, color.b, color.a])
    });
    imgbuf.save(&args.output)?;

    Ok(())
}
