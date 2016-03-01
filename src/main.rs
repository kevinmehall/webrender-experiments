use std::os::raw::c_void;

extern crate glutin;
use glutin::{Event, GlRequest, Api};

extern crate gleam;
use gleam::gl;

extern crate euclid;
use euclid::{Size2D, Point2D, Rect, Matrix4};

extern crate webrender;
extern crate webrender_traits;

struct RenderNotifier(glutin::WindowProxy);

impl webrender_traits::RenderNotifier for RenderNotifier {
    fn new_frame_ready(&mut self) {
        println!("New frame ready");
        self.0.wakeup_event_loop();
    }

    fn pipeline_size_changed(&mut self,
                             _pipeline_id: webrender_traits::PipelineId,
                             size: Option<Size2D<f32>>) {
        let size = size.unwrap_or(Size2D::zero());
        println!("Pipeline size changed: {:?}", size);
    }
}

fn main() {
    let mut size = Size2D::new(800, 600);

    let window = glutin::WindowBuilder::new()
        .with_title("WebRender".to_string())
        .with_dimensions(size.width, size.height)
        .with_gl(GlRequest::Specific(Api::OpenGl, (3, 2)))
        .with_multitouch()
        .with_stencil_buffer(8)
    .build().unwrap();

    unsafe { window.make_current().expect("Failed to make context current!") };

    gl::load_with(|s| window.get_proc_address(s) as *const c_void);

    let (mut webrender, webrender_sender) = webrender::Renderer::new(webrender::RendererOptions {
        device_pixel_ratio: 1.0,
        resource_path: "shaders".into(),
        enable_aa: false,
        enable_msaa: false,
        enable_profiler: true,
    });

    webrender.set_render_notifier(Box::new(RenderNotifier(window.create_window_proxy())));

    let webrender_api = webrender_sender.create_api();
    let pipeline_id = webrender_traits::PipelineId(0, 0);

    let mut mouse_pos = Point2D::new(0.0, 0.0);
    let mut needs_draw = true;
    let mut epoch = 0;
    draw(size, &webrender_api, epoch, pipeline_id, mouse_pos);
    webrender_api.set_root_pipeline(pipeline_id);

    'mainloop: loop {
        let mut render = false;
        // Wait for one event, and then also handle subsequent events received without blocking
        for event in window.wait_events().next().into_iter().chain(window.poll_events()) {
            println!("{:?}", event);

            match event {
                Event::Closed => break 'mainloop,
                Event::Resized(w, h) => {
                    size = Size2D::new(w, h);
                }

                Event::MouseMoved((x, y)) => {
                    mouse_pos = Point2D::new(x as f32, y as f32);
                    needs_draw = true;
                }

                Event::Refresh | Event::Awakened => {
                    render = true;
                }

                _ => (),
            };
        }

        if needs_draw {
            epoch += 1;
            draw(size, &webrender_api, epoch, pipeline_id, mouse_pos);
            needs_draw = false;
        }

        if render {
            webrender.update();
            webrender.render(size);
            window.swap_buffers().unwrap();
        }
    }
}

fn draw(size: Size2D<u32>,
        webrender_api: &webrender_traits::RenderApi,
        epoch: u32,
        pipeline_id: webrender_traits::PipelineId,
        mouse_pos: Point2D<f32>
    ) {
    let bounds = Rect::new(Point2D::new(0., 0.), Size2D::new(size.width as f32, size.height as f32));
    let epoch = webrender_traits::Epoch(epoch);
    let box_rect = Rect::new(Point2D::new(mouse_pos.x - 50., mouse_pos.y - 50.), Size2D::new(100., 100.));

    let mut stacking_context = webrender_traits::StackingContext::new(
        Some(webrender_traits::ScrollLayerId::Normal(pipeline_id, 0)),
        webrender_traits::ScrollPolicy::Scrollable,
        bounds, bounds,
        0,
        &Matrix4::identity(), &Matrix4::identity(),
        false,
        webrender_traits::MixBlendMode::Normal,
        vec![],
    );

    let mut display_list = webrender_traits::DisplayListBuilder::new();

    display_list.push_box_shadow(webrender_traits::StackingLevel::Content,
        box_rect,
        webrender_traits::ClipRegion::new(bounds, vec![]),
        box_rect,
        Point2D::new(5.0, 5.0),
        webrender_traits::ColorF::new(0., 0., 0., 0.5),
        5.,
        0.,
        0.,
        webrender_traits::BoxShadowClipMode::None);

    display_list.push_rect(webrender_traits::StackingLevel::Content,
        box_rect,
        webrender_traits::ClipRegion::new(bounds, vec![]),
        webrender_traits::ColorF::new(0., 0., 1., 1.));

    display_list.finalize();

    webrender_api.add_display_list(display_list, &mut stacking_context, pipeline_id, epoch);

    let stacking_context_id = webrender_api.add_stacking_context(stacking_context, pipeline_id, epoch);

    webrender_api.set_root_stacking_context(stacking_context_id, webrender_traits::ColorF::new(1., 1., 1., 1.), epoch, pipeline_id, bounds.size);
}
