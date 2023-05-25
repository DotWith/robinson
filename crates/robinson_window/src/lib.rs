use robinson_css::StyleSheet;
use robinson_dom::Node;
use state::State;
use winit::{
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod state;

pub async fn create_window(title: &str, root_node: &Node, stylesheets: &Vec<StyleSheet>) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();
    let mut state = State::new(&window, root_node, stylesheets).await;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { ref event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Resized(size) => state.resize(*size),
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                state.resize(**new_inner_size)
            },
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    // state,
                    virtual_keycode: Some(keycode),
                    ..
                },
                ..
            } => {
                match keycode {
                    VirtualKeyCode::P => {
                        // Make the pdf (temp keybind).
                        state.print_pdf().unwrap();
                    }
                    _ => {}
                }
            },
            _ => (),
        },
        Event::RedrawRequested(_) => state.render().unwrap(),
        _ => (),
    });
}
