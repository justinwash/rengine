use glam::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_pos_size(pos: Vec2, size: Vec2) -> Self {
        Self {
            x: pos.x,
            y: pos.y,
            width: size.x,
            height: size.y,
        }
    }

    pub fn left(&self) -> f32 {
        self.x
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y
    }

    pub fn top(&self) -> f32 {
        self.y + self.height
    }

    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    pub fn contains_point(&self, point: Vec2) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.bottom()
            && point.y <= self.top()
    }

    pub fn overlaps(&self, other: &Rect) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.bottom() < other.top()
            && self.top() > other.bottom()
    }
}
