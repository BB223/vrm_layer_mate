use std::time::Duration;

use glium::backend::Facade;
use glium::{Program, program};
use gtk::gdk::Display;
use gtk::{Application, ApplicationWindow, glib};
use gtk::{CssProvider, prelude::*};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use vrm_layer_mate::glium_gl_area::GliumGLArea;

const APP_ID: &str = "org.gtk_rs.HelloWorld1";

fn main() -> glib::ExitCode {
     // Load GL pointers from epoxy (GL context management library used by GTK).
    {
        #[cfg(target_os = "macos")]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.0.dylib") }.unwrap();
        #[cfg(all(unix, not(target_os = "macos")))]
        let library = unsafe { libloading::os::unix::Library::new("libepoxy.so.0") }.unwrap();
        #[cfg(windows)]
        let library = libloading::os::windows::Library::open_already_loaded("libepoxy-0.dll")
            .or_else(|_| libloading::os::windows::Library::open_already_loaded("epoxy-0.dll"))
            .unwrap();

        epoxy::load_with(|name| {
            unsafe { library.get::<_>(name.as_bytes()) }
                .map(|symbol| *symbol)
                .unwrap_or(std::ptr::null())
        });
    }

    let app = Application::builder().application_id(APP_ID).build();

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.run()
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string("window { background-color: transparent; }");

    gtk::style_context_add_provider_for_display(
        &Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application) {
    let glarea = GliumGLArea::default();
    // Create a window and set the title
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My GTK App")
        .child(&glarea)
        .build();

    window.init_layer_shell();
    window.set_namespace(Some("VRM Layer Mate"));
    window.set_layer(Layer::Bottom);

    let anchors = [
        (Edge::Left, true),
        (Edge::Right, true),
        (Edge::Top, true),
        (Edge::Bottom, true),
    ];
    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
    }
    window.set_exclusive_zone(0);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_decorated(false);

    window.present();

    let frame_time = Duration::new(0, 1_000_000_000 / 60);
    glib::source::timeout_add_local(frame_time, move || {
        glarea.queue_draw();
        glib::ControlFlow::Continue
    });
}

fn create_program<F>(display: &F) -> Program
where
    F: Facade,
{
    program!(display,
                320 es => {
            vertex: "
                #version 320 es
                uniform mat4 matrix;
                in vec2 position;
                in vec3 color;
                out vec3 vColor;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",
            fragment: "
                #version 320 es
                precision mediump float;
                in vec3 vColor;
                out vec4 f_color;
                void main() {
                    f_color = vec4(vColor, 1.0);
                }
            "
        }
    )
    .unwrap()
}
