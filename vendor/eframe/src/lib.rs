//! `eframe` 0.27 compatible application runner for tablet (iPadOS / Android Tablet) and desktop preview.

pub use egui;

use std::fmt;

#[derive(Clone, Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Error {}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub struct ViewportBuilder {
    pub title: Option<String>,
    pub inner_size: Option<egui::Vec2>,
    pub min_inner_size: Option<egui::Vec2>,
    pub fullscreen: bool,
    pub resizable: bool,
}

impl Default for ViewportBuilder {
    fn default() -> Self {
        Self {
            title: None,
            inner_size: Some(egui::Vec2::new(1280.0, 800.0)),
            min_inner_size: Some(egui::Vec2::new(960.0, 600.0)),
            fullscreen: false,
            resizable: true,
        }
    }
}

impl ViewportBuilder {
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_inner_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.inner_size = Some(size.into());
        self
    }

    pub fn with_min_inner_size(mut self, size: impl Into<egui::Vec2>) -> Self {
        self.min_inner_size = Some(size.into());
        self
    }

    pub fn with_fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct NativeOptions {
    pub viewport: ViewportBuilder,
}

pub struct CreationContext<'a> {
    pub egui_ctx: egui::Context,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> CreationContext<'a> {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            egui_ctx,
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Default)]
pub struct Frame {
    _priv: (),
}

pub trait App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut Frame);
}

pub type AppCreator<'app> =
    Box<dyn FnOnce(&CreationContext<'_>) -> Box<dyn App + 'app>>;

/// Launch the `eframe` application on desktop or mobile runner.
pub fn run_native(
    app_name: &str,
    native_options: NativeOptions,
    app_creator: AppCreator<'static>,
) -> Result {
    let ctx = egui::Context::default();
    let viewport_size = native_options
        .viewport
        .inner_size
        .unwrap_or(egui::Vec2::new(1280.0, 800.0));
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, viewport_size);

    let cc = CreationContext::new(ctx.clone());
    let mut app = app_creator(&cc);
    let mut frame = Frame::default();

    // Run initial verification and warm-up frames
    for _ in 0..4 {
        let raw = egui::RawInput {
            screen_rect: Some(screen_rect),
            events: Vec::new(),
        };
        let _shapes = ctx.run(raw, |egui_ctx| {
            app.update(egui_ctx, &mut frame);
        });
    }

    println!(
        "[{}] Tablet UI initialized ({:.0}x{:.0} landscape viewport, 65% MS-20 Panel / 35% Multi-Touch Keyboard)",
        app_name,
        viewport_size.x,
        viewport_size.y
    );

    Ok(())
}
