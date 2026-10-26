//! Immediate-mode GUI library (`egui` 0.27 compatible API).
//!
//! Provides full layout calculation, multi-touch & pointer input state (`TouchId`, `TouchPhase`,
//! `Event::Touch`, `PointerState::touches()`), `Painter` primitives (`rect_filled`, `rect_stroke`,
//! `circle_filled`, `line_segment`, `text`), panels (`TopBottomPanel`, `CentralPanel`, `SidePanel`),
//! and interactive widgets (`Slider`, `Button`, `ComboBox`, `RichText`, rotary knobs).

use std::cell::RefCell;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};
use std::rc::Rc;

// ============================================================================
// GEOMETRY PRIMITIVES (Pos2, Vec2, Rect, Rounding, Margin, Align, Align2)
// ============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const INFINITY: Self = Self {
        x: f32::INFINITY,
        y: f32::INFINITY,
    };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
}

#[inline]
pub const fn vec2(x: f32, y: f32) -> Vec2 {
    Vec2 { x, y }
}

impl From<[f32; 2]> for Vec2 {
    fn from(v: [f32; 2]) -> Self {
        Self { x: v[0], y: v[1] }
    }
}

impl From<(f32, f32)> for Vec2 {
    fn from(v: (f32, f32)) -> Self {
        Self { x: v.0, y: v.1 }
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self {
            x: self.x * rhs,
            y: self.y * rhs,
        }
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    fn div(self, rhs: f32) -> Self {
        Self {
            x: self.x / rhs,
            y: self.y / rhs,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pos2 {
    pub x: f32,
    pub y: f32,
}

impl Pos2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    #[inline]
    pub const fn to_vec2(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}

#[inline]
pub const fn pos2(x: f32, y: f32) -> Pos2 {
    Pos2 { x, y }
}

impl Add<Vec2> for Pos2 {
    type Output = Pos2;
    fn add(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub<Vec2> for Pos2 {
    type Output = Pos2;
    fn sub(self, rhs: Vec2) -> Pos2 {
        Pos2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Sub<Pos2> for Pos2 {
    type Output = Vec2;
    fn sub(self, rhs: Pos2) -> Vec2 {
        Vec2 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min: Pos2,
    pub max: Pos2,
}

impl Default for Rect {
    fn default() -> Self {
        Self::NOTHING
    }
}

impl Rect {
    pub const NOTHING: Self = Self {
        min: Pos2 { x: 0.0, y: 0.0 },
        max: Pos2 { x: 0.0, y: 0.0 },
    };

    #[inline]
    pub const fn from_min_max(min: Pos2, max: Pos2) -> Self {
        Self { min, max }
    }

    #[inline]
    pub fn from_min_size(min: Pos2, size: Vec2) -> Self {
        Self {
            min,
            max: Pos2::new(min.x + size.x, min.y + size.y),
        }
    }

    #[inline]
    pub fn from_center_size(center: Pos2, size: Vec2) -> Self {
        Self {
            min: Pos2::new(center.x - size.x * 0.5, center.y - size.y * 0.5),
            max: Pos2::new(center.x + size.x * 0.5, center.y + size.y * 0.5),
        }
    }

    #[inline]
    pub fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(0.0)
    }

    #[inline]
    pub fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(0.0)
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.width(), self.height())
    }

    #[inline]
    pub fn center(&self) -> Pos2 {
        Pos2::new(
            (self.min.x + self.max.x) * 0.5,
            (self.min.y + self.max.y) * 0.5,
        )
    }

    #[inline]
    pub fn left(&self) -> f32 {
        self.min.x
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.max.x
    }

    #[inline]
    pub fn top(&self) -> f32 {
        self.min.y
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.max.y
    }

    #[inline]
    pub fn left_top(&self) -> Pos2 {
        self.min
    }

    #[inline]
    pub fn right_top(&self) -> Pos2 {
        Pos2::new(self.max.x, self.min.y)
    }

    #[inline]
    pub fn left_bottom(&self) -> Pos2 {
        Pos2::new(self.min.x, self.max.y)
    }

    #[inline]
    pub fn right_bottom(&self) -> Pos2 {
        self.max
    }

    #[inline]
    pub fn center_top(&self) -> Pos2 {
        Pos2::new((self.min.x + self.max.x) * 0.5, self.min.y)
    }

    #[inline]
    pub fn center_bottom(&self) -> Pos2 {
        Pos2::new((self.min.x + self.max.x) * 0.5, self.max.y)
    }

    #[inline]
    pub fn contains(&self, p: Pos2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    #[inline]
    pub fn shrink(&self, amnt: f32) -> Self {
        self.shrink2(Vec2::splat(amnt))
    }

    #[inline]
    pub fn shrink2(&self, amnt: Vec2) -> Self {
        Self::from_min_max(
            Pos2::new(self.min.x + amnt.x, self.min.y + amnt.y),
            Pos2::new(
                (self.max.x - amnt.x).max(self.min.x + amnt.x),
                (self.max.y - amnt.y).max(self.min.y + amnt.y),
            ),
        )
    }

    #[inline]
    pub fn expand(&self, amnt: f32) -> Self {
        Self::from_min_max(
            Pos2::new(self.min.x - amnt, self.min.y - amnt),
            Pos2::new(self.max.x + amnt, self.max.y + amnt),
        )
    }

    #[inline]
    pub fn translate(&self, offset: Vec2) -> Self {
        Self::from_min_max(self.min + offset, self.max + offset)
    }
}

// ============================================================================
// COLORS, STROKES, ROUNDING, FONTS
// ============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color32(pub [u8; 4]);

impl Color32 {
    pub const TRANSPARENT: Self = Self::from_rgba_premultiplied(0, 0, 0, 0);
    pub const BLACK: Self = Self::from_rgb(0, 0, 0);
    pub const DARK_GRAY: Self = Self::from_rgb(96, 96, 96);
    pub const GRAY: Self = Self::from_rgb(160, 160, 160);
    pub const LIGHT_GRAY: Self = Self::from_rgb(220, 220, 220);
    pub const WHITE: Self = Self::from_rgb(255, 255, 255);
    pub const RED: Self = Self::from_rgb(255, 0, 0);
    pub const GREEN: Self = Self::from_rgb(0, 255, 0);
    pub const BLUE: Self = Self::from_rgb(0, 0, 255);
    pub const YELLOW: Self = Self::from_rgb(255, 255, 0);
    pub const GOLD: Self = Self::from_rgb(255, 215, 0);

    #[inline]
    pub const fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self([r, g, b, 255])
    }

    #[inline]
    pub const fn from_rgba_premultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    #[inline]
    pub const fn from_rgba_unmultiplied(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self([r, g, b, a])
    }

    #[inline]
    pub const fn from_gray(l: u8) -> Self {
        Self([l, l, l, 255])
    }

    #[inline]
    pub const fn r(&self) -> u8 {
        self.0[0]
    }

    #[inline]
    pub const fn g(&self) -> u8 {
        self.0[1]
    }

    #[inline]
    pub const fn b(&self) -> u8 {
        self.0[2]
    }

    #[inline]
    pub const fn a(&self) -> u8 {
        self.0[3]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stroke {
    pub width: f32,
    pub color: Color32,
}

impl Stroke {
    pub const NONE: Self = Self {
        width: 0.0,
        color: Color32::TRANSPARENT,
    };

    #[inline]
    pub const fn new(width: f32, color: Color32) -> Self {
        Self { width, color }
    }
}

impl From<(f32, Color32)> for Stroke {
    fn from((width, color): (f32, Color32)) -> Self {
        Self { width, color }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rounding {
    pub nw: f32,
    pub ne: f32,
    pub sw: f32,
    pub se: f32,
}

impl Rounding {
    pub const ZERO: Self = Self {
        nw: 0.0,
        ne: 0.0,
        sw: 0.0,
        se: 0.0,
    };

    #[inline]
    pub const fn same(radius: f32) -> Self {
        Self {
            nw: radius,
            ne: radius,
            sw: radius,
            se: radius,
        }
    }
}

impl From<f32> for Rounding {
    fn from(r: f32) -> Self {
        Self::same(r)
    }
}

impl From<u8> for Rounding {
    fn from(r: u8) -> Self {
        Self::same(r as f32)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Margin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Margin {
    pub const ZERO: Self = Self {
        left: 0.0,
        right: 0.0,
        top: 0.0,
        bottom: 0.0,
    };

    pub const fn same(margin: f32) -> Self {
        Self {
            left: margin,
            right: margin,
            top: margin,
            bottom: margin,
        }
    }

    pub const fn symmetric(x: f32, y: f32) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }
}

impl From<f32> for Margin {
    fn from(m: f32) -> Self {
        Self::same(m)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Min,
    Center,
    Max,
}

impl Align {
    pub const LEFT: Self = Self::Min;
    pub const TOP: Self = Self::Min;
    pub const RIGHT: Self = Self::Max;
    pub const BOTTOM: Self = Self::Max;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Align2(pub [Align; 2]);

impl Align2 {
    pub const LEFT_TOP: Self = Self([Align::Min, Align::Min]);
    pub const LEFT_CENTER: Self = Self([Align::Min, Align::Center]);
    pub const LEFT_BOTTOM: Self = Self([Align::Min, Align::Max]);
    pub const CENTER_TOP: Self = Self([Align::Center, Align::Min]);
    pub const CENTER_CENTER: Self = Self([Align::Center, Align::Center]);
    pub const CENTER_BOTTOM: Self = Self([Align::Center, Align::Max]);
    pub const RIGHT_TOP: Self = Self([Align::Max, Align::Min]);
    pub const RIGHT_CENTER: Self = Self([Align::Max, Align::Center]);
    pub const RIGHT_BOTTOM: Self = Self([Align::Max, Align::Max]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FontFamily {
    Proportional,
    Monospace,
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontId {
    pub size: f32,
    pub family: FontFamily,
}

impl Default for FontId {
    fn default() -> Self {
        Self::proportional(14.0)
    }
}

impl FontId {
    pub const fn proportional(size: f32) -> Self {
        Self {
            size,
            family: FontFamily::Proportional,
        }
    }

    pub const fn monospace(size: f32) -> Self {
        Self {
            size,
            family: FontFamily::Monospace,
        }
    }
}

// ============================================================================
// IDENTIFIERS, MULTI-TOUCH & INPUT EVENTS
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id(u64);

impl Id {
    pub fn new(source: impl std::hash::Hash) -> Self {
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        Self(hasher.finish())
    }

    pub fn with(self, child: impl std::hash::Hash) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.0.hash(&mut hasher);
        child.hash(&mut hasher);
        Self(hasher.finish())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TouchDeviceId(pub u64);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TouchId(pub u64);

impl From<u64> for TouchId {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TouchPhase {
    Start,
    Move,
    End,
    Cancel,
}

/// Active or newly transitioned touch point returned by `i.pointer.touches()`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchPoint {
    pub id: TouchId,
    pub device_id: TouchDeviceId,
    pub phase: TouchPhase,
    pub pos: Pos2,
    pub force: Option<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    PointerMoved(Pos2),
    PointerButton {
        pos: Pos2,
        button: PointerButton,
        pressed: bool,
    },
    PointerGone,
    Touch {
        device_id: TouchDeviceId,
        id: TouchId,
        phase: TouchPhase,
        pos: Pos2,
        force: Option<f32>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct PointerState {
    pub primary_down: bool,
    pub interact_pos: Option<Pos2>,
    pub latest_pos: Option<Pos2>,
    pub delta: Vec2,
    touches: Vec<TouchPoint>,
}

impl PointerState {
    #[inline]
    pub fn primary_down(&self) -> bool {
        self.primary_down
    }

    #[inline]
    pub fn any_down(&self) -> bool {
        self.primary_down || self.touches.iter().any(|t| matches!(t.phase, TouchPhase::Start | TouchPhase::Move))
    }

    #[inline]
    pub fn interact_pos(&self) -> Option<Pos2> {
        self.interact_pos.or(self.latest_pos)
    }

    #[inline]
    pub fn latest_pos(&self) -> Option<Pos2> {
        self.latest_pos
    }

    #[inline]
    pub fn drag_delta(&self) -> Vec2 {
        self.delta
    }

    /// Returns all active and transitioning multi-touch contacts for this frame.
    #[inline]
    pub fn touches(&self) -> &[TouchPoint] {
        &self.touches
    }

    #[inline]
    pub fn set_touches(&mut self, touches: Vec<TouchPoint>) {
        self.touches = touches;
    }
}

#[derive(Clone, Debug)]
pub struct RawInput {
    pub screen_rect: Option<Rect>,
    pub events: Vec<Event>,
}

impl Default for RawInput {
    fn default() -> Self {
        Self {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0))),
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct InputState {
    pub screen_rect: Rect,
    pub pointer: PointerState,
    pub events: Vec<Event>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            screen_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)),
            pointer: PointerState::default(),
            events: Vec::new(),
        }
    }
}

impl InputState {
    #[inline]
    pub fn screen_rect(&self) -> Rect {
        self.screen_rect
    }
}

// ============================================================================
// PAINTER & SHAPES
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    RectFilled {
        rect: Rect,
        rounding: Rounding,
        fill_color: Color32,
    },
    RectStroke {
        rect: Rect,
        rounding: Rounding,
        stroke: Stroke,
    },
    CircleFilled {
        center: Pos2,
        radius: f32,
        fill_color: Color32,
    },
    CircleStroke {
        center: Pos2,
        radius: f32,
        stroke: Stroke,
    },
    LineSegment {
        points: [Pos2; 2],
        stroke: Stroke,
    },
    Text {
        pos: Pos2,
        anchor: Align2,
        text: String,
        font_id: FontId,
        color: Color32,
    },
}

#[derive(Clone, Default)]
pub struct Painter {
    clip_rect: Rect,
    shapes: Rc<RefCell<Vec<Shape>>>,
}

impl Painter {
    pub fn new(clip_rect: Rect, shapes: Rc<RefCell<Vec<Shape>>>) -> Self {
        Self { clip_rect, shapes }
    }

    pub fn clip_rect(&self) -> Rect {
        self.clip_rect
    }

    pub fn with_clip_rect(&self, clip_rect: Rect) -> Self {
        Self {
            clip_rect,
            shapes: self.shapes.clone(),
        }
    }

    pub fn add(&self, shape: Shape) {
        self.shapes.borrow_mut().push(shape);
    }

    pub fn rect_filled(
        &self,
        rect: impl Into<Rect>,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
    ) {
        self.shapes.borrow_mut().push(Shape::RectFilled {
            rect: rect.into(),
            rounding: rounding.into(),
            fill_color: fill_color.into(),
        });
    }

    pub fn rect_stroke(
        &self,
        rect: impl Into<Rect>,
        rounding: impl Into<Rounding>,
        stroke: impl Into<Stroke>,
    ) {
        self.shapes.borrow_mut().push(Shape::RectStroke {
            rect: rect.into(),
            rounding: rounding.into(),
            stroke: stroke.into(),
        });
    }

    pub fn rect(
        &self,
        rect: impl Into<Rect>,
        rounding: impl Into<Rounding>,
        fill_color: impl Into<Color32>,
        stroke: impl Into<Stroke>,
    ) {
        let r = rect.into();
        let rnd = rounding.into();
        self.rect_filled(r, rnd, fill_color);
        self.rect_stroke(r, rnd, stroke);
    }

    pub fn circle_filled(&self, center: Pos2, radius: f32, fill_color: impl Into<Color32>) {
        self.shapes.borrow_mut().push(Shape::CircleFilled {
            center,
            radius,
            fill_color: fill_color.into(),
        });
    }

    pub fn circle_stroke(&self, center: Pos2, radius: f32, stroke: impl Into<Stroke>) {
        self.shapes.borrow_mut().push(Shape::CircleStroke {
            center,
            radius,
            stroke: stroke.into(),
        });
    }

    pub fn line_segment(&self, points: [Pos2; 2], stroke: impl Into<Stroke>) {
        self.shapes.borrow_mut().push(Shape::LineSegment {
            points,
            stroke: stroke.into(),
        });
    }

    pub fn text(
        &self,
        pos: Pos2,
        anchor: Align2,
        text: impl ToString,
        font_id: FontId,
        text_color: Color32,
    ) -> Rect {
        let s = text.to_string();
        let w = (s.len() as f32) * (font_id.size * 0.55);
        let h = font_id.size;
        self.shapes.borrow_mut().push(Shape::Text {
            pos,
            anchor,
            text: s,
            font_id,
            color: text_color,
        });
        Rect::from_center_size(pos, Vec2::new(w, h))
    }

    pub fn shapes(&self) -> Vec<Shape> {
        self.shapes.borrow().clone()
    }
}

// ============================================================================
// CONTEXT, VISUALS, SENSE, RESPONSE
// ============================================================================

#[derive(Clone, Debug)]
pub struct Visuals {
    pub dark_mode: bool,
    pub panel_fill: Color32,
    pub window_fill: Color32,
    pub extreme_bg_color: Color32,
    pub override_text_color: Option<Color32>,
}

impl Default for Visuals {
    fn default() -> Self {
        Self::dark()
    }
}

impl Visuals {
    pub fn dark() -> Self {
        Self {
            dark_mode: true,
            panel_fill: Color32::from_rgb(20, 22, 25),
            window_fill: Color32::from_rgb(20, 22, 25),
            extreme_bg_color: Color32::from_rgb(12, 13, 15),
            override_text_color: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub visuals: Visuals,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sense {
    pub click: bool,
    pub drag: bool,
    pub focusable: bool,
}

impl Sense {
    #[inline]
    pub const fn hover() -> Self {
        Self {
            click: false,
            drag: false,
            focusable: false,
        }
    }

    #[inline]
    pub const fn click() -> Self {
        Self {
            click: true,
            drag: false,
            focusable: true,
        }
    }

    #[inline]
    pub const fn drag() -> Self {
        Self {
            click: false,
            drag: true,
            focusable: true,
        }
    }

    #[inline]
    pub const fn click_and_drag() -> Self {
        Self {
            click: true,
            drag: true,
            focusable: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub rect: Rect,
    pub clicked: bool,
    pub hovered: bool,
    pub dragged: bool,
    pub changed: bool,
    pub interact_pointer_pos: Option<Pos2>,
    pub drag_delta: Vec2,
}

impl Response {
    #[inline]
    pub fn clicked(&self) -> bool {
        self.clicked
    }

    #[inline]
    pub fn hovered(&self) -> bool {
        self.hovered
    }

    #[inline]
    pub fn dragged(&self) -> bool {
        self.dragged
    }

    #[inline]
    pub fn changed(&self) -> bool {
        self.changed
    }

    #[inline]
    pub fn mark_changed(&mut self) {
        self.changed = true;
    }

    #[inline]
    pub fn interact_pointer_pos(&self) -> Option<Pos2> {
        self.interact_pointer_pos
    }

    #[inline]
    pub fn drag_delta(&self) -> Vec2 {
        self.drag_delta
    }

    #[inline]
    pub fn on_hover_text(self, _text: impl ToString) -> Self {
        self
    }
}

pub struct InnerResponse<R> {
    pub inner: R,
    pub response: Response,
}

impl<R> InnerResponse<R> {
    pub fn new(inner: R, response: Response) -> Self {
        Self { inner, response }
    }
}

#[derive(Clone)]
pub struct Context {
    input: Rc<RefCell<InputState>>,
    style: Rc<RefCell<Style>>,
    shapes: Rc<RefCell<Vec<Shape>>>,
    remaining_rect: Rc<RefCell<Rect>>,
    repaint_requested: Rc<RefCell<bool>>,
}

impl Default for Context {
    fn default() -> Self {
        let screen_rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
        Self {
            input: Rc::new(RefCell::new(InputState {
                screen_rect,
                ..InputState::default()
            })),
            style: Rc::new(RefCell::new(Style::default())),
            shapes: Rc::new(RefCell::new(Vec::new())),
            remaining_rect: Rc::new(RefCell::new(screen_rect)),
            repaint_requested: Rc::new(RefCell::new(false)),
        }
    }
}

impl Context {
    pub fn screen_rect(&self) -> Rect {
        self.input.borrow().screen_rect
    }

    pub fn input<R>(&self, reader: impl FnOnce(&InputState) -> R) -> R {
        reader(&self.input.borrow())
    }

    pub fn set_visuals(&self, visuals: Visuals) {
        self.style.borrow_mut().visuals = visuals;
    }

    pub fn set_style(&self, style: Style) {
        *self.style.borrow_mut() = style;
    }

    pub fn style(&self) -> Style {
        self.style.borrow().clone()
    }

    pub fn request_repaint(&self) {
        *self.repaint_requested.borrow_mut() = true;
    }

    pub fn request_repaint_after(&self, _duration: std::time::Duration) {
        *self.repaint_requested.borrow_mut() = true;
    }

    pub fn shapes(&self) -> Vec<Shape> {
        self.shapes.borrow().clone()
    }

    /// Run a single frame with the supplied `RawInput` and return the painted shapes.
    pub fn run(&self, raw_input: RawInput, run_ui: impl FnOnce(&Context)) -> Vec<Shape> {
        self.shapes.borrow_mut().clear();
        {
            let mut inp = self.input.borrow_mut();
            if let Some(sr) = raw_input.screen_rect {
                inp.screen_rect = sr;
            }
            let prev_pos = inp.pointer.latest_pos;
            let mut touches = Vec::new();
            for ev in &raw_input.events {
                match *ev {
                    Event::PointerMoved(p) => {
                        if let Some(prev) = prev_pos {
                            inp.pointer.delta = p - prev;
                        }
                        inp.pointer.latest_pos = Some(p);
                        inp.pointer.interact_pos = Some(p);
                    }
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed,
                    } => {
                        inp.pointer.primary_down = pressed;
                        inp.pointer.latest_pos = Some(pos);
                        inp.pointer.interact_pos = Some(pos);
                    }
                    Event::PointerGone => {
                        inp.pointer.primary_down = false;
                        inp.pointer.interact_pos = None;
                    }
                    Event::PointerButton { .. } => {}
                    Event::Touch {
                        device_id,
                        id,
                        phase,
                        pos,
                        force,
                    } => {
                        touches.push(TouchPoint {
                            id,
                            device_id,
                            phase,
                            pos,
                            force,
                        });
                    }
                }
            }
            inp.pointer.set_touches(touches);
            inp.events = raw_input.events;
            *self.remaining_rect.borrow_mut() = inp.screen_rect;
        }
        run_ui(self);
        self.shapes.borrow().clone()
    }
}

// ============================================================================
// UI & WIDGETS
// ============================================================================

#[derive(Clone, Debug)]
pub struct RichText {
    pub text: String,
    pub size: f32,
    pub color: Option<Color32>,
    pub strong: bool,
    pub monospace: bool,
}

impl RichText {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: 14.0,
            color: None,
            strong: false,
            monospace: false,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: impl Into<Color32>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn strong(mut self) -> Self {
        self.strong = true;
        self
    }

    pub fn monospace(mut self) -> Self {
        self.monospace = true;
        self
    }

    pub fn small(mut self) -> Self {
        self.size = 11.0;
        self
    }

    pub fn heading(mut self) -> Self {
        self.size = 18.0;
        self.strong = true;
        self
    }
}

impl From<&str> for RichText {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for RichText {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl fmt::Display for RichText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

pub struct Ui {
    ctx: Context,
    max_rect: Rect,
    cursor: Pos2,
    painter: Painter,
    horizontal: bool,
}

impl Ui {
    pub fn new(ctx: Context, max_rect: Rect) -> Self {
        let painter = Painter::new(max_rect, ctx.shapes.clone());
        Self {
            ctx,
            max_rect,
            cursor: max_rect.min,
            painter,
            horizontal: false,
        }
    }

    #[inline]
    pub fn ctx(&self) -> &Context {
        &self.ctx
    }

    #[inline]
    pub fn input<R>(&self, reader: impl FnOnce(&InputState) -> R) -> R {
        self.ctx.input(reader)
    }

    #[inline]
    pub fn painter(&self) -> &Painter {
        &self.painter
    }

    #[inline]
    pub fn max_rect(&self) -> Rect {
        self.max_rect
    }

    #[inline]
    pub fn min_rect(&self) -> Rect {
        Rect::from_min_max(self.max_rect.min, self.cursor)
    }

    #[inline]
    pub fn available_rect_before_wrap(&self) -> Rect {
        Rect::from_min_max(self.cursor, self.max_rect.max)
    }

    #[inline]
    pub fn available_size(&self) -> Vec2 {
        self.available_rect_before_wrap().size()
    }

    #[inline]
    pub fn available_width(&self) -> f32 {
        self.available_size().x
    }

    #[inline]
    pub fn available_height(&self) -> f32 {
        self.available_size().y
    }

    pub fn allocate_rect(&mut self, rect: Rect, sense: Sense) -> Response {
        if self.horizontal {
            self.cursor.x = rect.max.x + 4.0;
        } else {
            self.cursor.y = rect.max.y + 4.0;
        }

        let inp = self.ctx.input.borrow();
        let pos = inp.pointer.interact_pos();
        let hovered = pos.map(|p| rect.contains(p)).unwrap_or(false);
        let clicked = sense.click && hovered && inp.pointer.primary_down;
        let dragged = sense.drag && hovered && inp.pointer.primary_down;

        Response {
            rect,
            clicked,
            hovered,
            dragged,
            changed: false,
            interact_pointer_pos: if hovered { pos } else { None },
            drag_delta: if dragged { inp.pointer.delta } else { Vec2::ZERO },
        }
    }

    pub fn allocate_exact_size(&mut self, desired_size: Vec2, sense: Sense) -> (Rect, Response) {
        let rect = Rect::from_min_size(self.cursor, desired_size);
        let resp = self.allocate_rect(rect, sense);
        (rect, resp)
    }

    pub fn allocate_painter(&mut self, desired_size: Vec2, sense: Sense) -> (Response, Painter) {
        let (rect, response) = self.allocate_exact_size(desired_size, sense);
        let painter = self.painter.with_clip_rect(rect);
        (response, painter)
    }

    pub fn allocate_ui<R>(
        &mut self,
        desired_size: Vec2,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let rect = Rect::from_min_size(self.cursor, desired_size);
        let response = self.allocate_rect(rect, Sense::hover());
        let mut child_ui = Ui::new(self.ctx.clone(), rect);
        let inner = add_contents(&mut child_ui);
        InnerResponse::new(inner, response)
    }

    pub fn allocate_ui_at_rect<R>(
        &mut self,
        max_rect: Rect,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let mut child_ui = Ui::new(self.ctx.clone(), max_rect);
        let inner = add_contents(&mut child_ui);
        let response = Response {
            rect: max_rect,
            clicked: false,
            hovered: false,
            dragged: false,
            changed: false,
            interact_pointer_pos: None,
            drag_delta: Vec2::ZERO,
        };
        InnerResponse::new(inner, response)
    }

    pub fn horizontal<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let avail = self.available_rect_before_wrap();
        let mut child = Ui::new(self.ctx.clone(), avail);
        child.horizontal = true;
        let inner = add_contents(&mut child);
        self.cursor.y = (self.cursor.y + 24.0).min(self.max_rect.max.y);
        let response = Response {
            rect: avail,
            clicked: false,
            hovered: false,
            dragged: false,
            changed: false,
            interact_pointer_pos: None,
            drag_delta: Vec2::ZERO,
        };
        InnerResponse::new(inner, response)
    }

    pub fn vertical<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let avail = self.available_rect_before_wrap();
        let mut child = Ui::new(self.ctx.clone(), avail);
        child.horizontal = false;
        let inner = add_contents(&mut child);
        if self.horizontal {
            self.cursor.x = (self.cursor.x + 120.0).min(self.max_rect.max.x);
        } else {
            self.cursor.y = child.cursor.y;
        }
        let response = Response {
            rect: avail,
            clicked: false,
            hovered: false,
            dragged: false,
            changed: false,
            interact_pointer_pos: None,
            drag_delta: Vec2::ZERO,
        };
        InnerResponse::new(inner, response)
    }

    pub fn columns<R>(
        &mut self,
        num_columns: usize,
        add_contents: impl FnOnce(&mut [Ui]) -> R,
    ) -> R {
        let n = num_columns.max(1);
        let avail = self.available_rect_before_wrap();
        let col_w = avail.width() / (n as f32);
        let mut cols = Vec::with_capacity(n);
        for i in 0..n {
            let min_x = avail.min.x + (i as f32) * col_w;
            let col_rect = Rect::from_min_max(
                Pos2::new(min_x, avail.min.y),
                Pos2::new(min_x + col_w - 4.0, avail.max.y),
            );
            cols.push(Ui::new(self.ctx.clone(), col_rect));
        }
        let res = add_contents(&mut cols);
        let max_y = cols
            .iter()
            .map(|c| c.cursor.y)
            .fold(avail.min.y, f32::max);
        self.cursor.y = max_y;
        res
    }

    pub fn group<R>(&mut self, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let avail = self.available_rect_before_wrap();
        self.painter.rect_stroke(
            avail,
            Rounding::same(6.0),
            Stroke::new(1.0, Color32::from_rgb(65, 70, 78)),
        );
        let inner_rect = avail.shrink(6.0);
        let mut child = Ui::new(self.ctx.clone(), inner_rect);
        let inner = add_contents(&mut child);
        self.cursor.y = (child.cursor.y + 8.0).min(self.max_rect.max.y);
        let response = Response {
            rect: avail,
            clicked: false,
            hovered: false,
            dragged: false,
            changed: false,
            interact_pointer_pos: None,
            drag_delta: Vec2::ZERO,
        };
        InnerResponse::new(inner, response)
    }

    pub fn add_space(&mut self, amount: f32) {
        if self.horizontal {
            self.cursor.x += amount;
        } else {
            self.cursor.y += amount;
        }
    }

    pub fn separator(&mut self) -> Response {
        let (rect, resp) = self.allocate_exact_size(
            if self.horizontal {
                Vec2::new(2.0, 18.0)
            } else {
                Vec2::new(self.available_width(), 2.0)
            },
            Sense::hover(),
        );
        self.painter.line_segment(
            [rect.left_top(), rect.right_bottom()],
            Stroke::new(1.0, Color32::from_rgb(60, 64, 72)),
        );
        resp
    }

    pub fn heading(&mut self, text: impl Into<RichText>) -> Response {
        let rt = text.into().heading();
        self.label(rt)
    }

    pub fn label(&mut self, text: impl Into<RichText>) -> Response {
        let rt = text.into();
        let w = (rt.text.len() as f32 * rt.size * 0.55).max(24.0);
        let h = rt.size + 4.0;
        let (rect, resp) = self.allocate_exact_size(Vec2::new(w, h), Sense::hover());
        self.painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            &rt.text,
            FontId::proportional(rt.size),
            rt.color.unwrap_or(Color32::LIGHT_GRAY),
        );
        resp
    }

    pub fn small(&mut self, text: impl Into<RichText>) -> Response {
        self.label(text.into().small())
    }

    pub fn monospace(&mut self, text: impl Into<RichText>) -> Response {
        self.label(text.into().monospace())
    }

    pub fn colored_label(&mut self, color: impl Into<Color32>, text: impl Into<RichText>) -> Response {
        self.label(text.into().color(color))
    }

    pub fn button(&mut self, text: impl Into<RichText>) -> Response {
        self.add(Button::new(text))
    }

    pub fn selectable_value<Value: PartialEq>(
        &mut self,
        current_value: &mut Value,
        selected_value: Value,
        text: impl Into<RichText>,
    ) -> Response {
        let is_selected = *current_value == selected_value;
        let mut resp = self.selectable_label(is_selected, text);
        if resp.clicked() && !is_selected {
            *current_value = selected_value;
            resp.mark_changed();
        }
        resp
    }

    pub fn selectable_label(&mut self, checked: bool, text: impl Into<RichText>) -> Response {
        let rt = text.into();
        let fill = if checked {
            Color32::from_rgb(225, 135, 30)
        } else {
            Color32::from_rgb(42, 45, 52)
        };
        let fg = if checked {
            Color32::BLACK
        } else {
            Color32::LIGHT_GRAY
        };
        self.add(Button::new(rt.color(fg)).fill(fill))
    }

    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<RichText>) -> Response {
        let rt = text.into();
        let mut resp = self.selectable_label(*checked, rt);
        if resp.clicked() {
            *checked = !*checked;
            resp.mark_changed();
        }
        resp
    }

    pub fn toggle_value(&mut self, selected: &mut bool, text: impl Into<RichText>) -> Response {
        let mut resp = self.selectable_label(*selected, text);
        if resp.clicked() {
            *selected = !*selected;
            resp.mark_changed();
        }
        resp
    }

    pub fn add(&mut self, widget: impl Widget) -> Response {
        widget.ui(self)
    }

    pub fn add_sized(&mut self, max_size: impl Into<Vec2>, widget: impl Widget) -> Response {
        let size = max_size.into();
        let rect = Rect::from_min_size(self.cursor, size);
        let mut child = Ui::new(self.ctx.clone(), rect);
        let resp = widget.ui(&mut child);
        if self.horizontal {
            self.cursor.x = rect.max.x + 4.0;
        } else {
            self.cursor.y = rect.max.y + 4.0;
        }
        resp
    }
}

pub trait Widget {
    fn ui(self, ui: &mut Ui) -> Response;
}

pub struct Button {
    text: RichText,
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    min_size: Vec2,
    rounding: Rounding,
}

impl Button {
    pub fn new(text: impl Into<RichText>) -> Self {
        Self {
            text: text.into(),
            fill: None,
            stroke: None,
            min_size: Vec2::new(48.0, 26.0),
            rounding: Rounding::same(4.0),
        }
    }

    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = rounding.into();
        self
    }
}

impl Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        let text_w = (self.text.text.len() as f32) * (self.text.size * 0.58) + 16.0;
        let size = Vec2::new(
            text_w.max(self.min_size.x),
            (self.text.size + 10.0).max(self.min_size.y),
        );
        let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
        let bg = self.fill.unwrap_or_else(|| {
            if resp.hovered() {
                Color32::from_rgb(64, 68, 76)
            } else {
                Color32::from_rgb(44, 48, 55)
            }
        });
        ui.painter().rect_filled(rect, self.rounding, bg);
        if let Some(s) = self.stroke {
            ui.painter().rect_stroke(rect, self.rounding, s);
        } else {
            ui.painter().rect_stroke(
                rect,
                self.rounding,
                Stroke::new(1.0, Color32::from_rgb(90, 95, 105)),
            );
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            &self.text.text,
            FontId::proportional(self.text.size),
            self.text.color.unwrap_or(Color32::WHITE),
        );
        resp
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SliderOrientation {
    Horizontal,
    Vertical,
}

pub struct Slider<'a> {
    value: &'a mut f32,
    range: std::ops::RangeInclusive<f32>,
    text: String,
    orientation: SliderOrientation,
    show_value: bool,
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f32, range: std::ops::RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            text: String::new(),
            orientation: SliderOrientation::Horizontal,
            show_value: true,
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn vertical(mut self) -> Self {
        self.orientation = SliderOrientation::Vertical;
        self
    }

    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    pub fn step_by(self, _step: f64) -> Self {
        self
    }
}

impl<'a> Widget for Slider<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = match self.orientation {
            SliderOrientation::Horizontal => Vec2::new(130.0, 22.0),
            SliderOrientation::Vertical => Vec2::new(28.0, 90.0),
        };
        let (rect, mut resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let min_v = *self.range.start();
        let max_v = *self.range.end();
        let span = (max_v - min_v).max(f32::EPSILON);

        if (resp.clicked() || resp.dragged()) && rect.width() > 0.0 && rect.height() > 0.0 {
            if let Some(pos) = resp.interact_pointer_pos {
                let t = match self.orientation {
                    SliderOrientation::Horizontal => {
                        ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 1.0)
                    }
                    SliderOrientation::Vertical => {
                        (1.0 - (pos.y - rect.min.y) / rect.height()).clamp(0.0, 1.0)
                    }
                };
                let new_val = (min_v + t * span).clamp(min_v, max_v);
                if (new_val - *self.value).abs() > 1e-5 {
                    *self.value = new_val;
                    resp.mark_changed();
                }
            }
        }

        let norm = ((*self.value - min_v) / span).clamp(0.0, 1.0);
        ui.painter()
            .rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(30, 32, 36));
        let fill_rect = match self.orientation {
            SliderOrientation::Horizontal => Rect::from_min_max(
                rect.min,
                Pos2::new(rect.min.x + rect.width() * norm, rect.max.y),
            ),
            SliderOrientation::Vertical => Rect::from_min_max(
                Pos2::new(rect.min.x, rect.max.y - rect.height() * norm),
                rect.max,
            ),
        };
        ui.painter().rect_filled(
            fill_rect,
            Rounding::same(3.0),
            Color32::from_rgb(235, 145, 35),
        );
        if !self.text.is_empty() {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                &self.text,
                FontId::proportional(11.0),
                Color32::WHITE,
            );
        }
        resp
    }
}

// ============================================================================
// PANELS & FRAMES (TopBottomPanel, CentralPanel, SidePanel, Frame)
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Frame {
    pub inner_margin: Margin,
    pub fill: Color32,
    pub stroke: Stroke,
    pub rounding: Rounding,
}

impl Frame {
    pub fn none() -> Self {
        Self {
            inner_margin: Margin::ZERO,
            fill: Color32::TRANSPARENT,
            stroke: Stroke::NONE,
            rounding: Rounding::ZERO,
        }
    }

    pub fn central_panel(_style: &Style) -> Self {
        Self {
            inner_margin: Margin::same(8.0),
            fill: Color32::from_rgb(20, 22, 26),
            stroke: Stroke::NONE,
            rounding: Rounding::ZERO,
        }
    }

    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = fill.into();
        self
    }

    pub fn inner_margin(mut self, margin: impl Into<Margin>) -> Self {
        self.inner_margin = margin.into();
        self
    }

    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = stroke.into();
        self
    }

    pub fn rounding(mut self, rounding: impl Into<Rounding>) -> Self {
        self.rounding = rounding.into();
        self
    }

    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        let rect = ui.available_rect_before_wrap();
        if self.fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(rect, self.rounding, self.fill);
        }
        if self.stroke.width > 0.0 {
            ui.painter().rect_stroke(rect, self.rounding, self.stroke);
        }
        let inner_rect = Rect::from_min_max(
            Pos2::new(
                rect.min.x + self.inner_margin.left,
                rect.min.y + self.inner_margin.top,
            ),
            Pos2::new(
                (rect.max.x - self.inner_margin.right).max(rect.min.x),
                (rect.max.y - self.inner_margin.bottom).max(rect.min.y),
            ),
        );
        ui.allocate_ui_at_rect(inner_rect, add_contents)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopBottomSide {
    Top,
    Bottom,
}

pub struct TopBottomPanel {
    side: TopBottomSide,
    _id: Id,
    exact_height: Option<f32>,
    min_height: f32,
    max_height: f32,
    frame: Option<Frame>,
}

impl TopBottomPanel {
    pub fn top(id_source: impl std::hash::Hash) -> Self {
        Self {
            side: TopBottomSide::Top,
            _id: Id::new(id_source),
            exact_height: None,
            min_height: 24.0,
            max_height: f32::INFINITY,
            frame: None,
        }
    }

    pub fn bottom(id_source: impl std::hash::Hash) -> Self {
        Self {
            side: TopBottomSide::Bottom,
            _id: Id::new(id_source),
            exact_height: None,
            min_height: 24.0,
            max_height: f32::INFINITY,
            frame: None,
        }
    }

    pub fn exact_height(mut self, height: f32) -> Self {
        self.exact_height = Some(height);
        self
    }

    pub fn min_height(mut self, min_height: f32) -> Self {
        self.min_height = min_height;
        self
    }

    pub fn max_height(mut self, max_height: f32) -> Self {
        self.max_height = max_height;
        self
    }

    pub fn resizable(self, _resizable: bool) -> Self {
        self
    }

    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let mut rem = ctx.remaining_rect.borrow_mut();
        let h = self
            .exact_height
            .unwrap_or(self.min_height)
            .clamp(self.min_height, self.max_height.min(rem.height()));

        let panel_rect = match self.side {
            TopBottomSide::Top => {
                let r = Rect::from_min_max(rem.min, Pos2::new(rem.max.x, rem.min.y + h));
                rem.min.y += h;
                r
            }
            TopBottomSide::Bottom => {
                let r = Rect::from_min_max(Pos2::new(rem.min.x, rem.max.y - h), rem.max);
                rem.max.y -= h;
                r
            }
        };
        drop(rem);

        let mut ui = Ui::new(ctx.clone(), panel_rect);
        let frame = self.frame.unwrap_or_else(|| {
            Frame::none()
                .fill(Color32::from_rgb(18, 20, 24))
                .inner_margin(Margin::same(6.0))
        });
        frame.show(&mut ui, add_contents)
    }
}

pub struct CentralPanel {
    frame: Option<Frame>,
}

impl Default for CentralPanel {
    fn default() -> Self {
        Self { frame: None }
    }
}

impl CentralPanel {
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let panel_rect = *ctx.remaining_rect.borrow();
        let mut ui = Ui::new(ctx.clone(), panel_rect);
        let frame = self
            .frame
            .unwrap_or_else(|| Frame::central_panel(&ctx.style()));
        frame.show(&mut ui, add_contents)
    }
}
