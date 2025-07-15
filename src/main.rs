use glium::Frame;
use gtk::gdk::Display;
use gtk::{glib, Application, ApplicationWindow, GLArea};
use gtk::{CssProvider, prelude::*};
use gtk4_glium::GtkFacade;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const APP_ID: &str = "org.gtk_rs.HelloWorld1";

fn main() -> glib::ExitCode {
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
    let glarea = GLArea::builder().vexpand(true).build();
    let facade = GtkFacade::from_glarea(&glarea).unwrap();
    glarea.connect_render(move |glarea, ctx| {
        let context = facade.get_context();
        glib::Propagation::Stop
    });
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
}
