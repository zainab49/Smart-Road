// player.rs — Single keyboard-controlled character.

use crate::animation::AnimState;

/// Movement speed in pixels per simulation tick.
pub const PLAYER_SPEED: f32 = 3.0;

pub struct Player {
    pub x: f32,
    pub y: f32,
    /// Which sprite sheet to use (0 = v1.png … 6 = v7.png).
    pub sprite_index: u8,
    pub anim: AnimState,
    /// True while any movement key is held — used to freeze the frame
    /// counter when the character is standing still.
    pub moving: bool,
}

impl Player {
    /// Spawn a player at world position `(x, y)` using sprite sheet `sprite_index`.
    pub fn new(x: f32, y: f32, sprite_index: u8) -> Self {
        Player {
            x,
            y,
            sprite_index,
            anim: AnimState::new(180.0), // start facing down (toward camera)
            moving: false,
        }
    }

    /// Call once per tick.
    ///
    /// `dx` / `dy` are raw axis values: −1.0, 0.0, or +1.0.
    ///
    /// * Normalises diagonal movement so speed is consistent.
    /// * Translates the player position.
    /// * Advances the animation only while moving (idle = frame frozen).
    pub fn update(&mut self, dx: f32, dy: f32, window_w: f32, window_h: f32) {
        self.moving = dx != 0.0 || dy != 0.0;

        if self.moving {
            // Normalise so diagonal speed == cardinal speed.
            let len = (dx * dx + dy * dy).sqrt();
            let nx  = dx / len;
            let ny  = dy / len;

            self.x = (self.x + nx * PLAYER_SPEED).clamp(0.0, window_w);
            self.y = (self.y + ny * PLAYER_SPEED).clamp(0.0, window_h);

            // Convert the movement vector to the project's angle convention:
            //   0° = up, 90° = right, 180° = down, 270° = left.
            let angle = ny.atan2(nx).to_degrees() + 90.0;
            self.anim.tick(angle);
        }
        // While idle: direction row preserved, frame column stays put.
    }
}
