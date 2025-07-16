pub mod glium_gl_area {
    use gtk::{gdk, glib, prelude::*};

    glib::wrapper! {
        pub struct GliumGLArea(ObjectSubclass<imp::GliumGLArea>)
            @extends gtk::GLArea, gtk::Widget,
            @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
    }

    impl Default for GliumGLArea {
        fn default() -> Self {
            glib::Object::builder().build()
        }
    }

    impl GliumGLArea {
        pub fn new() -> Self {
            glib::Object::builder().build()
        }
    }

    unsafe impl glium::backend::Backend for GliumGLArea {
        fn swap_buffers(&self) -> Result<(), glium::SwapBuffersError> {
            Ok(())
        }

        unsafe fn get_proc_address(&self, symbol: &str) -> *const std::os::raw::c_void {
            epoxy::get_proc_addr(symbol)
        }

        fn get_framebuffer_dimensions(&self) -> (u32, u32) {
            let scale = self.scale_factor();
            let width = self.width();
            let height = self.height();
            ((width * scale) as u32, (height * scale) as u32)
        }

        fn resize(&self, new_size: (u32, u32)) {
            self.set_size_request(new_size.0 as i32, new_size.1 as i32)
        }

        fn is_current(&self) -> bool {
            match self.context() {
                Some(context) => gdk::GLContext::current() == Some(context),
                None => false,
            }
        }

        unsafe fn make_current(&self) {
            GLAreaExt::make_current(self)
        }
    }

    mod imp {
        use std::{cell::RefCell, path::PathBuf, rc::Rc};

        use cgmath::SquareMatrix;
        use glium::{Surface, program};
        use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

        struct VrmModel {
            context: Rc<glium::backend::Context>,
            program: glium::Program,
            model: PathBuf,
        }

        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 3],
            normal: [f32; 3],
            tex_coords: [f32; 2],
            color: [f32; 4],
        }

        glium::implement_vertex!(Vertex, position, normal, tex_coords, color);

        impl VrmModel {
            fn new(context: Rc<glium::backend::Context>) -> Self {
                let program = glium::program!(&context,
                    320 es => {
                        vertex: "
                            #version 320 es
                            uniform mat4 matrix;
                            in vec3 position;
                            in vec3 normal;
                            in vec2 tex_coords;
                            in vec4 color;
                            out vec3 frag_normal;
                            out vec2 frag_tex_coords;
                            out vec4 frag_color;
                            void main() {
                                frag_normal = normal;
                                frag_tex_coords = tex_coords;
                                frag_color = color;
                                gl_Position = matrix * vec4(position, 1.0);
                            }
                        ",

                        fragment: "
                            #version 320 es
                            precision mediump float;
                            in vec3 frag_normal;
                            in vec2 frag_tex_coords;
                            in vec4 frag_color;
                            out vec4 color;
                            void main() {
                                vec3 light_dir = normalize(vec3(0.0, 0.0, 1.0)); // from camera
                                float brightness = max(dot(normalize(frag_normal), light_dir), 0.0);
                                vec3 lit_color = frag_color.rgb * brightness;
                                color = vec4(lit_color, frag_color.a);
                            }
                        "
                    },
                )
                .unwrap();
                Self {
                    context: context,
                    program: program,
                    model: PathBuf::new(),
                }
            }
            fn draw(&self) {
                let mut frame = glium::Frame::new(
                    self.context.clone(),
                    self.context.get_framebuffer_dimensions(),
                );
                frame.clear_color(0.0, 0.0, 0.0, 0.0);
                let (doc, buffers, _) =
                    gltf::import("/home/bzell/personal/vrm_layer_mate/Corset.glb").expect("file");
                for camera in doc.cameras() {
                    println!("Camera: {:?}", camera);
                }
                for mesh in doc.meshes() {
                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                        // Extract vertex data
                        let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
                        let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
                        let tex_coords: Vec<[f32; 2]> = reader
                            .read_tex_coords(0)
                            .map(|tc| tc.into_f32().collect())
                            .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);
                        let colors: Vec<[f32; 4]> = reader
                            .read_colors(0)
                            .map(|c| match c {
                                gltf::mesh::util::ReadColors::RgbaF32(iter) => iter.collect(),
                                _ => vec![[1.0, 1.0, 1.0, 1.0]; positions.len()],
                            })
                            .unwrap_or_else(|| vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]);
                        let vertices: Vec<Vertex> = positions
                            .into_iter()
                            .zip(normals)
                            .zip(tex_coords)
                            .zip(colors)
                            .map(|(((pos, norm), uv), color)| Vertex {
                                position: pos,
                                normal: norm,
                                tex_coords: uv,
                                color: color,
                            })
                            .collect();
                        let indices: Vec<u32> = reader
                            .read_indices()
                            .map(|i| i.into_u32().collect())
                            .unwrap_or_else(|| (0..vertices.len() as u32).collect());
                        let vertex_buffer =
                            glium::VertexBuffer::new(&self.context, &vertices).unwrap();
                        let index_buffer = glium::IndexBuffer::new(
                            &self.context,
                            glium::index::PrimitiveType::TrianglesList,
                            &indices,
                        )
                        .unwrap();
                        let aspect_ratio = 1920 as f32 / 1047 as f32;
                        let projection =
                            cgmath::perspective(cgmath::Deg(40.0), aspect_ratio, 0.1, 100.0);
                        let view = cgmath::Matrix4::look_at_rh(
                            cgmath::Point3::new(0.0, 0.05, 0.2),
                            cgmath::Point3::new(0.0, 0.05, 0.0),
                            cgmath::Vector3::unit_y(),
                        );
                        let model = cgmath::Matrix4::identity();
                        let mvp = projection * view * model;
                        let matrix: [[f32; 4]; 4] = mvp.into();
                        let uniforms = glium::uniform! {matrix: matrix};
                        frame
                            .draw(
                                &vertex_buffer,
                                &index_buffer,
                                &self.program,
                                &uniforms,
                                &Default::default(),
                            )
                            .unwrap();
                    }
                }
                frame.finish().unwrap();
            }
        }

        #[derive(Default)]
        pub struct GliumGLArea {
            vrm_model: RefCell<Option<VrmModel>>,
        }
        #[glib::object_subclass]
        impl ObjectSubclass for GliumGLArea {
            const NAME: &'static str = "GliumGLArea";
            type Type = super::GliumGLArea;
            type ParentType = gtk::GLArea;
        }
        impl ObjectImpl for GliumGLArea {}
        impl WidgetImpl for GliumGLArea {
            fn realize(&self) {
                self.parent_realize();

                let widget = self.obj();
                if widget.error().is_some() {
                    return;
                }

                let context = unsafe {
                    glium::backend::Context::new(widget.clone(), true, Default::default())
                }
                .unwrap();
                *self.vrm_model.borrow_mut() = Some(VrmModel::new(context));
            }

            fn unrealize(&self) {
                *self.vrm_model.borrow_mut() = None;

                self.parent_unrealize();
            }
        }
        impl GLAreaImpl for GliumGLArea {
            fn render(&self, _context: &gdk::GLContext) -> glib::Propagation {
                self.vrm_model.borrow().as_ref().unwrap().draw();
                glib::Propagation::Stop
            }
        }
    }
}
