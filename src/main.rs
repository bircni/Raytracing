use glium::{
    glutin::{
        dpi::PhysicalSize,
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
        ContextBuilder,
    },
    implement_vertex, uniform, Surface,
};

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
}

implement_vertex!(Vertex, position);

pub fn main() {
    let window_builder = WindowBuilder::new()
        .with_title("TrayRacer!")
        .with_resizable(true)
        .with_min_inner_size(PhysicalSize::new(800, 600));
    let context_builder = ContextBuilder::new();
    let event_loop = EventLoop::new();

    let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();

    event_loop.run(move |e, _, c| match e {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *c = ControlFlow::Exit;
        }
        Event::WindowEvent { .. } => {}
        Event::RedrawRequested(_) => {
            let mut frame = display.draw();
            draw_gradient_background(&display, &mut frame);
            frame.finish().unwrap();
        }
        _ => {}
    });
}

fn draw_gradient_background(display: &glium::Display, target: &mut glium::Frame) {
    let vertex_buffer = glium::VertexBuffer::new(
        display,
        &[
            Vertex {
                position: [-1.0, -1.0],
            },
            Vertex {
                position: [-1.0, 1.0],
            },
            Vertex {
                position: [1.0, -1.0],
            },
            Vertex {
                position: [1.0, 1.0],
            },
        ],
    )
    .unwrap();

    let indices = glium::index::NoIndices(glium::index::PrimitiveType::TriangleStrip);

    let shader_program = create_shader(display);

    target
        .draw(
            &vertex_buffer,
            &indices,
            &shader_program,
            &uniform! {},
            &Default::default(),
        )
        .unwrap();
}

fn create_shader(display: &glium::Display) -> glium::Program {
    let vertex_shader_source = r#"
        #version 330 core

        in vec2 position;

        void main() {
            gl_Position = vec4(position, 0.0, 1.0);
        }
    "#;

    let _shader_vertical = r#"
        #version 330 core
        out vec4 color;

        void main() {
            float t = (gl_FragCoord.x / 600.0);
            color = vec4(1.0, t, 1.0 - t, 1.0);
        }
    "#;

    let _shader_horizontal = r#"
        #version 330 core
        out vec4 color;

        void main() {
            float t = (gl_FragCoord.y / 600.0);
            color = vec4(1.0, t, 1.0 - t, 1.0);
        }
    "#;

    let _shader_rainbow = r#"
    #version 330 core
    out vec4 color;

    void main() {
        float t = gl_FragCoord.x / 800.0; // Adjust the divisor for the gradient width
        vec3 rainbowColor;

        if (t < 0.14) {
            rainbowColor = vec3(1.0, 0.0, 0.0); // Red
        } else if (t < 0.28) {
            rainbowColor = mix(vec3(1.0, 0.0, 0.0), vec3(1.0, 0.5, 0.0), smoothstep(0.14, 0.28, t));
        } else if (t < 0.43) {
            rainbowColor = mix(vec3(1.0, 0.5, 0.0), vec3(1.0, 1.0, 0.0), smoothstep(0.28, 0.43, t));
        } else if (t < 0.57) {
            rainbowColor = mix(vec3(1.0, 1.0, 0.0), vec3(0.0, 1.0, 0.0), smoothstep(0.43, 0.57, t));
        } else if (t < 0.71) {
            rainbowColor = mix(vec3(0.0, 1.0, 0.0), vec3(0.0, 0.0, 1.0), smoothstep(0.57, 0.71, t));
        } else if (t < 0.85) {
            rainbowColor = mix(vec3(0.0, 0.0, 1.0), vec3(0.29, 0.0, 0.51), smoothstep(0.71, 0.85, t));
        } else {
            rainbowColor = mix(vec3(0.29, 0.0, 0.51), vec3(0.5, 0.0, 0.5), smoothstep(0.85, 1.0, t));
        }

        color = vec4(rainbowColor, 1.0);
    }
    "#;

    let shader_radial = r#"
    #version 330 core
    out vec4 color;

    void main() {
        // Calculate the distance from the center of the screen
        vec2 center = vec2(400.0, 300.0); // Center of the screen (adjust as needed)
        float distance = length(gl_FragCoord.xy - center) / 400.0; // Adjust divisor for gradient size

        // The gradient will be from the center (0.0) to the edge (1.0)
        color = vec4(0.0, distance, 1.0 - distance, 1.0);
    }
    "#;

    glium::Program::from_source(display, vertex_shader_source, shader_radial, None).unwrap()
}
