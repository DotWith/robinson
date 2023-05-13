use glium::{glutin::{event_loop::{EventLoop, ControlFlow}, window::WindowBuilder, event::{Event, WindowEvent}, ContextBuilder}, Display};
use robinson_css::StyleSheet;
use robinson_dom::Node;
use state::State;

mod state;

pub fn create_window(title: &str, root_node: &Node, stylesheets: &Vec<StyleSheet>) {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(title);
    let context = ContextBuilder::new();
    let display = Display::new(window, context, &event_loop).unwrap();
    let mut state = State::new(display, root_node, stylesheets);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                ref event,
                ..
            } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => state.resize(*size),
                _ => (),
            },
            Event::RedrawRequested(_) | Event::MainEventsCleared => state.render(),
            _ => (),
        }
    });
}