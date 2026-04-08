// animation.rs - Sprite-sheet animation for 3x4 character sheets.
//
// Row 0: Up
// Row 1: Right
// Row 2: Down
// Row 3: Left
//
// The source textures are 3 columns x 4 rows.

use sdl2::rect::Rect;

/// Number of frame columns per row.
pub const FRAME_COLS: u8 = 3;
/// Number of direction rows.
pub const FRAME_ROWS: u8 = 4;
/// Ticks to hold each frame before advancing.
pub const ANIM_TICKS_PER_FRAME: u32 = 8;

/// The 4 direction rows in the sprite sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimDir {
    Up = 0,
    Right = 1,
    Down = 2,
    Left = 3,
}

impl AnimDir {
    /// Convert heading angle to one of 4 cardinal directions.
    ///
    /// Convention:
    /// 0 = up, 90 = right, 180 = down, 270 = left.
    pub fn from_angle(angle: f32) -> Self {
        let a = ((angle % 360.0) + 360.0) % 360.0;

        if a < 45.0 || a >= 315.0 {
            AnimDir::Up
        } else if a < 135.0 {
            AnimDir::Right
        } else if a < 225.0 {
            AnimDir::Down
        } else {
            AnimDir::Left
        }
    }
}

/// Runtime animation state for one actor.
#[derive(Clone, Debug)]
pub struct AnimState {
    /// Current facing direction (row index).
    pub dir: AnimDir,
    /// Current frame (column index).
    pub frame: u8,
    /// Tick accumulator for frame stepping.
    timer: u32,
}

impl AnimState {
    /// Initialize animation using the current heading.
    pub fn new(initial_angle: f32) -> Self {
        Self {
            dir: AnimDir::from_angle(initial_angle),
            frame: 0,
            timer: 0,
        }
    }

    /// Advance one simulation tick.
    pub fn tick(&mut self, angle: f32) {
        self.dir = AnimDir::from_angle(angle);

        self.timer += 1;
        if self.timer >= ANIM_TICKS_PER_FRAME {
            self.timer = 0;
            self.frame = (self.frame + 1) % FRAME_COLS;
        }
    }

    /// Source rectangle inside the sprite sheet for the current frame.
    pub fn src_rect(&self, sheet_w: u32, sheet_h: u32) -> Rect {
        let frame_w = (sheet_w / FRAME_COLS as u32).max(1);
        let frame_h = (sheet_h / FRAME_ROWS as u32).max(1);
        Rect::new(
            self.frame as i32 * frame_w as i32,
            self.dir as i32 * frame_h as i32,
            frame_w,
            frame_h,
        )
    }
}
