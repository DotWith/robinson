mod error;

use std::path::Path;

use clap::Parser;
use error::Result;
use robinson_css::StyleSheet;
use robinson_dom::Dom;
use robinson_net::Client;
use robinson_window::create_window;

/// A toy web rendering engine
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Website URL
    #[arg(long, default_value = "examples/test.html")]
    website: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Create the network connection.
    let client = Client::default();

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
                        if let Some(_rel) = eee
                            .attributes
                            .get("rel")
                            .filter(|&rel| rel == &Some(String::from("stylesheet")))
                        {
                            if let Some(href) = eee.attributes.get("href").cloned() {
                                let css_url = href.unwrap();
                                let css_path = Path::new(&css_url);
                                let html_path = Path::new(&args.website);
                                let html_url = html_path.parent().unwrap();
                                let connected_path = html_url.join(css_path);
                                let css_str = connected_path.to_str().unwrap();
                                let css = client.get_to_string(client.get_url(css_str)?).await?;
                                stylesheet_links.push(css);
                            }
                        }
                    } else if eee.name == "style" {
                        if let Some(first_child) = eee.children.first() {
                            if let Some(text) = first_child.text() {
                                stylesheet_links.push(text.to_string());
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
        let stylesheet = StyleSheet::parse(&css)?;
        stylesheets.push(stylesheet);
    }

    // Render to window
    create_window("Robinson", root_node, &stylesheets).await;

    Ok(())
}
