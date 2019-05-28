use azul::{azul_dependencies::glium, prelude::*, resources::FontSource};
use image;
use std::{
    cell::RefCell,
    process,
    sync::{atomic, Arc},
    thread,
};
use tokio::{
    prelude::*,
    sync::{mpsc, oneshot},
};

struct DataModel {
    dom: String,
    render: RefCell<mpsc::UnboundedSender<image::DynamicImage>>,
    close: RefCell<oneshot::Receiver<()>>,
    completed: Arc<atomic::AtomicBool>,
}

impl Layout for DataModel {
    fn layout(&self, layout_info: LayoutInfo<DataModel>) -> Dom<DataModel> {
        let mut close_receiver = self.close.borrow_mut();
        if close_receiver.try_recv().is_ok() {
            self.completed.store(true, atomic::Ordering::Relaxed);

            // This should only happen within a thread
            process::exit(0);
        }

        let image: glium::texture::RawImage2d<u8> = layout_info
            .window
            .read_only_window()
            .inner
            .read_front_buffer();

        let mut is_empty_image = true;
        for datum in image.data.iter() {
            if *datum != 0 {
                is_empty_image = false;
                break;
            }
        }

        if !is_empty_image {
            let image =
                image::ImageBuffer::from_raw(image.width, image.height, image.data.into_owned())
                    .unwrap();
            let image = image::DynamicImage::ImageRgba8(image).flipv();

            if self.render.borrow_mut().try_send(image).is_err() {
                // TODO: Do something intelligent like logging properly
            };
        } else {
            // TODO: Do something like logging when image is empty
        }

        // TODO: Get updated DOM string dynamically
        Dom::from_xml(&self.dom, &mut XmlComponentMap::default()).unwrap()
    }
}

// TODO: Update assets and resources dynamically
pub fn spawn_dom<T: Into<String> + Clone + Send + 'static>(
    dom: T,
    render: mpsc::UnboundedSender<image::DynamicImage>,
    close: oneshot::Receiver<()>,
    css: Option<T>,
    system_fonts: Option<Vec<T>>,
) -> Box<dyn Future<Item = (), Error = ()> + Send> {
    let completed = Arc::new(atomic::AtomicBool::new(false));
    let done = Arc::clone(&completed);

    let task = || {
        let data_model = DataModel {
            dom: dom.into(),
            render: RefCell::new(render),
            close: RefCell::new(close),
            completed,
        };
        let app_config = AppConfig {
            enable_logging: None,
            log_file_path: None,
            enable_visual_panic_hook: false,
            enable_logging_on_panic: false,
            enable_tab_navigation: true,
            renderer_type: RendererType::default(),
            debug_state: DebugState::default(),
            background_color: ColorU {
                r: 255,
                g: 255,
                b: 255,
                a: 0,
            },
        };
        let mut app = App::new(data_model, app_config).unwrap();

        if let Some(system_fonts) = system_fonts {
            for system_font in system_fonts {
                let font: String = system_font.clone().into();
                let font_id = app.add_css_font_id(font.clone());
                let font_source = FontSource::System(font.clone());
                font_source.get_bytes().unwrap();
                app.add_font(font_id, font_source);
            }
        }

        let css = if let Some(css) = css {
            css::override_native(&css.into()).unwrap()
        } else {
            css::native()
        };

        let window = app
            .create_window(WindowCreateOptions::default(), css)
            .unwrap();

        let _blah = app.run(window);
    };

    Box::new(future::lazy(|| {
        thread::spawn(task);
        future::loop_fn(done, |c| {
            if c.load(atomic::Ordering::Relaxed) {
                Ok(future::Loop::Break(c))
            } else {
                Ok(future::Loop::Continue(c))
            }
        })
        .map(|_| ())
    }))
}
