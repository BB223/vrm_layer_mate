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
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };

        use cgmath::SquareMatrix;
        use glium::{
            IndexBuffer, Surface, VertexBuffer, program,
            texture::{RawImage2d, SrgbTexture2d},
        };
        use gtk::{gdk, glib, prelude::*, subclass::prelude::*};

        struct VrmModel {
            context: Rc<glium::backend::Context>,
            program: glium::Program,
            meshes: Vec<MeshInstance>,
        }

        #[derive(Copy, Clone)]
        struct Vertex {
            a_position: [f32; 3],
            a_uv: [f32; 2],
        }

        struct MeshInstance {
            vertex_buffer: VertexBuffer<Vertex>,
            index_buffer: IndexBuffer<u32>,
            texture: SrgbTexture2d,
        }

        glium::implement_vertex!(Vertex, a_position, a_uv);

        impl VrmModel {
            fn new(context: Rc<glium::backend::Context>) -> Self {
                let program = glium::program!(&context,
                    320 es => {
                        vertex: include_str!("unlit.vert.glsl"),
                        fragment: include_str!("unlit.frag.glsl")
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
                let (doc, buffers, images) =
                    gltf::import("/home/bzell/personal/vrm_layer_mate/Corset.glb").expect("file");
                let mut meshes = Vec::new();

                for mesh in doc.meshes() {
                    for primitive in mesh.primitives() {
                        let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                        // Extract vertex data
                        let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
                        let tex_coords = reader.read_tex_coords(0).unwrap().into_f32();
                        // let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
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
                            // .zip(normals)
                            // .zip(colors)
                            .map(|(pos, uv)| Vertex {
                                a_position: pos,
                                a_uv: uv,
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

                        let base_color_texture = primitive
                            .material()
                            .pbr_metallic_roughness()
                            .base_color_texture()
                            .unwrap();
                        let image_index = base_color_texture.texture().source().index();
                        let image_data = &images[image_index];
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
                        let raw_image = RawImage2d::from_raw_rgba_reversed(&rgba_pixels, dims);
                        let texture =
                            glium::texture::SrgbTexture2d::new(context, raw_image).unwrap();

                        println!(
                            "Loaded {} indices, {} vertices",
                            index_buffer.len(),
                            vertex_buffer.len()
                        );
                        meshes.push(MeshInstance {
                            vertex_buffer,
                            index_buffer,
                            texture,
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
                frame.clear_color_and_depth((0.0, 0.0, 0.0, 0.01), 1.0);

                // let (doc, buffers, images) =
                //     gltf::import("/home/bzell/personal/vrm_layer_mate/Corset.glb").expect("file");
                // let mut meshes = Vec::new();

                // for mesh in doc.meshes() {
                //     for primitive in mesh.primitives() {
                //         let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                //         // Extract vertex data
                //         let positions: Vec<[f32; 3]> = reader.read_positions().unwrap().collect();
                //         let tex_coords = reader.read_tex_coords(0).unwrap().into_f32();
                //         // let normals: Vec<[f32; 3]> = reader.read_normals().unwrap().collect();
                //         // let colors: Vec<[f32; 4]> = reader
                //         //     .read_colors(0)
                //         //     .map(|c| match c {
                //         //         gltf::mesh::util::ReadColors::RgbaF32(iter) => iter.collect(),
                //         //         _ => vec![[1.0, 1.0, 1.0, 1.0]; positions.len()],
                //         //     })
                //         //     .unwrap_or_else(|| vec![[1.0, 1.0, 1.0, 1.0]; positions.len()]);
                //         let vertices: Vec<Vertex> = positions
                //             .into_iter()
                //             .zip(tex_coords)
                //             // .zip(normals)
                //             // .zip(colors)
                //             .map(|(pos, uv)| Vertex {
                //                 a_position: pos,
                //                 a_uv: uv,
                //             })
                //             .collect();
                //         let indices: Vec<u32> = reader.read_indices().unwrap().into_u32().collect();
                //         let vertex_buffer =
                //             glium::VertexBuffer::new(&self.context, &vertices).unwrap();
                //         let index_buffer = glium::IndexBuffer::new(
                //             &self.context,
                //             glium::index::PrimitiveType::TrianglesList,
                //             &indices,
                //         )
                //         .unwrap();

                //         let base_color_texture = primitive
                //             .material()
                //             .pbr_metallic_roughness()
                //             .base_color_texture()
                //             .unwrap();
                //         let image_index = base_color_texture.texture().source().index();
                //         let image_data = &images[image_index];
                //         let dims = (image_data.width, image_data.height);
                //         // If not RGBA, convert to RGBA (just to be safe)
                //         let rgba_pixels = match image_data.format {
                //             gltf::image::Format::R8G8B8A8 => image_data.pixels.clone(),
                //             gltf::image::Format::R8G8B8 => {
                //                 // Expand RGB to RGBA
                //                 image_data
                //                     .pixels
                //                     .chunks(3)
                //                     .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255]) // Add alpha = 255
                //                     .collect()
                //             }
                //             _ => panic!("Unsupported pixel format: {:?}", image_data.format),
                //         };
                //         let raw_image = RawImage2d::from_raw_rgba_reversed(&rgba_pixels, dims);
                //         let texture =
                //             glium::texture::SrgbTexture2d::new(&self.context, raw_image).unwrap();

                //         println!(
                //             "Loaded {} indices, {} vertices",
                //             index_buffer.len(),
                //             vertex_buffer.len()
                //         );
                //         meshes.push(MeshInstance {
                //             vertex_buffer,
                //             index_buffer,
                //             texture,
                //         });
                //     }
                // }

                let aspect_ratio = 1920_f32 / 1047_f32;
                let projection: [[f32; 4]; 4] =
                    cgmath::perspective(cgmath::Deg(45.0), aspect_ratio, 0.1, 100.0).into();
                let view: [[f32; 4]; 4] = cgmath::Matrix4::look_at_rh(
                    cgmath::Point3::new(0.0, 0.0, 0.1),
                    cgmath::Point3::new(0.0, 0.0, 0.0),
                    cgmath::Vector3::unit_y(),
                )
                .into();
                let model: [[f32; 4]; 4] = cgmath::Matrix4::identity().into();
                println!("Rendering frame with {} meshes", self.meshes.len());

                let params = glium::DrawParameters {
                    depth: glium::Depth {
                        test: glium::DepthTest::IfLess,
                        write: true,
                        ..Default::default()
                    },
                    backface_culling: glium::BackfaceCullingMode::CullCounterClockwise,
                    ..Default::default()
                };
                for mesh in &self.meshes {
                    let uniforms = glium::uniform! { u_model: model, u_view: view, u_proj: projection, u_texture: &mesh.texture };
                    println!(
                        "Drawing {} indices, {} vertices",
                        mesh.index_buffer.len(),
                        mesh.vertex_buffer.len()
                    );

                    frame
                        .draw(
                            &mesh.vertex_buffer,
                            &mesh.index_buffer,
                            &self.program,
                            &uniforms,
                            &params,
                        )
                        .unwrap();
                }
                frame.finish().unwrap();
            }
        }

        #[derive(Default)]
        pub struct GliumGLArea {
            vrm_model: RefCell<Option<VrmModel>>,
            frame_counter: Rc<Cell<u32>>,
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
                let count = self.frame_counter.get();
                println!("Frame {count}");
                self.frame_counter.set(count + 1);
                glib::Propagation::Stop
            }
        }
    }
}
