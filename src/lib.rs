pub mod glium_gl_area {
    use gtk::{gdk, glib, prelude::*};

    glib::wrapper! {
        pub struct GliumGLArea(ObjectSubclass<imp::GliumGLAreaImp>)
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
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };

        use cgmath::{InnerSpace, Matrix, SquareMatrix};
        use glium::{
            CapabilitiesSource, IndexBuffer, Surface, Texture2d, VertexBuffer,
            backend::{Backend, Facade},
            program,
            texture::{RawImage2d, SrgbTexture2d},
        };
        use gtk::{GLArea, gdk, glib, prelude::*, subclass::prelude::*};

        struct VrmModel {
            context: Rc<glium::backend::Context>,
            program: glium::Program,
            meshes: Vec<MeshInstance>,
        }

        #[derive(Copy, Clone)]
        struct Vertex {
            position: [f32; 3],
            normal: [f32; 3],
            tex_coord: [f32; 2],
        }

        struct MeshInstance {
            vertex_buffer: VertexBuffer<Vertex>,
            index_buffer: IndexBuffer<u32>,
            image_data: gltf::image::Data,
        }

        glium::implement_vertex!(Vertex, position, normal, tex_coord);

        impl VrmModel {
            fn new(context: Rc<glium::backend::Context>) -> Self {
                let program = glium::program!(&context,
                    320 es => {
                        vertex: include_str!("shaders/simple.vert.glsl"),
                        fragment: include_str!("shaders/mtoon.frag.glsl")
                    },
                )
                .unwrap();
                let meshes = Self::load_model(&context);
                Self {
                    context,
                    program,
                    meshes,
                }
            }
            fn load_model(context: &Rc<glium::backend::Context>) -> Vec<MeshInstance> {
                let (doc, buffers, images) = gltf::import("models/girl.glb").unwrap();
                let mut meshes = Vec::new();

                for mesh in doc.meshes() {
                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        // Extract vertex data
                        let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
                        let tex_coords = reader.read_tex_coords(0).unwrap().into_f32();
                        let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
                        // let colors: Vec<[f32; 4]> = reader
                        //     .read_colors(0)
                        //     .map(|c| match c {
                        //         gltf::mesh::util::ReadColors::RgbaF32(iter) => iter.collect(),
                        //         _ => vec![[1.0, 1.0, 1.0, 1.0]; positions.len()],
                        //     })
                        //     .unwrap_or_else(|| vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]);
                        let vertices: Vec<Vertex> = positions
                            .into_iter()
                            .zip(tex_coords)
                            .zip(normals)
                            .map(|((pos, uv), normal)| Vertex {
                                position: pos,
                                tex_coord: uv,
                                normal: normal,
                            })
                            .collect();
                        let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
                        let vertex_buffer = glium::VertexBuffer::new(context, &vertices).unwrap();
                        let index_buffer = glium::IndexBuffer::new(
                            context,
                            glium::index::PrimitiveType::TrianglesList,
                            &indices,
                        )
                        .unwrap();

                        let material = primitive.material();
                        let pbr = material.pbr_metallic_roughness();
                        let base_color_texture = pbr.base_color_texture().unwrap();
                        let image_index = base_color_texture.texture().index();
                        let image_data = images.get(image_index).unwrap().clone();

                        println!(
                            "Loaded {} indices, {} vertices",
                            index_buffer.len(),
                            vertex_buffer.len()
                        );
                        meshes.push(MeshInstance {
                            vertex_buffer,
                            index_buffer,
                            image_data,
                        });
                    }
                }
                meshes
            }
            fn draw(&self) {
                let mut frame = glium::Frame::new(
                    self.context.clone(),
                    self.context.get_framebuffer_dimensions(),
                );
                frame.clear_color(0.0, 0.0, 0.0, 0.0);

                let aspect_ratio = 1920_f32 / 1080_f32;
                let projection = cgmath::perspective(cgmath::Deg(45.0), aspect_ratio, 0.1, 100.0);
                let eye = cgmath::Point3::new(0.0, 1.0, -2.5);
                let view = cgmath::Matrix4::look_at_rh(
                    eye,
                    cgmath::Point3::new(0.0, 1.0, 0.0),
                    cgmath::Vector3::unit_y(),
                );
                let model = cgmath::Matrix4::identity();

                let model_view = view * model;
                let mv3 = cgmath::Matrix3::new(
                    model_view.x.x,
                    model_view.x.y,
                    model_view.x.z,
                    model_view.y.x,
                    model_view.y.y,
                    model_view.y.z,
                    model_view.z.x,
                    model_view.z.y,
                    model_view.z.z,
                );
                let normal_matrix: [[f32; 3]; 3] = mv3.invert().unwrap().transpose().into();

                let light_dir: [f32; 3] = cgmath::Vector3::new(2.0, 1.0, -2.5).into();

                let model_matrix: [[f32; 4]; 4] = model.into();
                let view_matrix: [[f32; 4]; 4] = view.into();
                let projection_matrix: [[f32; 4]; 4] = projection.into();
                let camera_position: [f32; 3] = eye.into();

                for mesh in &self.meshes {
                    let image_data = &mesh.image_data;
                    let dims = (image_data.width, image_data.height);
                    // If not RGBA, convert to RGBA (just to be safe)
                    let rgba_pixels = match image_data.format {
                        gltf::image::Format::R8G8B8A8 => image_data.pixels.clone(),
                        gltf::image::Format::R8G8B8 => {
                            // Expand RGB to RGBA
                            image_data
                                .pixels
                                .chunks(3)
                                .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]) // Add alpha = 255
                                .collect()
                        }
                        _ => panic!("Unsupported pixel format: {:?}", image_data.format),
                    };
                    let raw_image = RawImage2d::from_raw_rgba(rgba_pixels, dims);
                    let texture = glium::texture::Texture2d::new(&self.context, raw_image).unwrap();

                    let uniforms = glium::uniform! { modelMatrix: model_matrix, viewMatrix: view_matrix, projectionMatrix: projection_matrix, normalMatrix: normal_matrix, cameraPosition: camera_position, utexture: texture, lightDir: light_dir };

                    frame
                        .draw(
                            &mesh.vertex_buffer,
                            &mesh.index_buffer,
                            &self.program,
                            &uniforms,
                            &Default::default(),
                        )
                        .unwrap();
                }
                frame.finish().unwrap();
            }
        }

        #[derive(Default)]
        pub struct GliumGLAreaImp {
            vrm_model: RefCell<Option<VrmModel>>,
            frame_counter: Rc<Cell<u32>>,
        }
        #[glib::object_subclass]
        impl ObjectSubclass for GliumGLAreaImp {
            const NAME: &'static str = "GliumGLArea";
            type Type = super::GliumGLArea;
            type ParentType = gtk::GLArea;
        }
        impl ObjectImpl for GliumGLAreaImp {}
        impl WidgetImpl for GliumGLAreaImp {
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
        impl GLAreaImpl for GliumGLAreaImp {
            fn render(&self, _context: &gdk::GLContext) -> glib::Propagation {
                if let Some(model) = self.vrm_model.borrow().as_ref() {
                    model.draw()
                }
                let count = self.frame_counter.get();
                println!("Frame {count}");
                self.frame_counter.set(count + 1);
                glib::Propagation::Stop
            }
        }
    }
}
