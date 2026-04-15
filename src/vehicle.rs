// vehicle.rs - Vehicle model, kinematics, and route following.

use std::time::Instant;

use crate::animation::AnimState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    Left,
    Straight,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Speed {
    Slow = 1,
    Medium = 2,
    Fast = 3,
}

impl Speed {
    pub fn pixels_per_tick(self) -> f32 {
        match self {
            Speed::Slow => 1.5,
            Speed::Medium => 2.5,
            Speed::Fast => 4.0,
        }
    }

    pub fn downshift(self) -> Speed {
        match self {
            Speed::Fast => Speed::Medium,
            Speed::Medium => Speed::Slow,
            Speed::Slow => Speed::Slow,
        }
    }

    pub fn upshift(self) -> Speed {
        match self {
            Speed::Slow => Speed::Medium,
            Speed::Medium => Speed::Fast,
            Speed::Fast => Speed::Fast,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Waypoint {
    pub x: f32,
    pub y: f32,
}

pub struct Vehicle {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub speed: Speed,
    pub direction: Direction,
    pub route: Route,
    pub waypoints: Vec<Waypoint>,
    pub waypoint_index: usize,
    pub entry_time: Instant,
    pub exit_time: Option<Instant>,
    pub active: bool,
    pub in_intersection: bool,
    pub reservation_id: Option<u32>,
    pub sprite_index: u8,
    pub anim: AnimState,
}

impl Vehicle {
    pub fn new(
        id: u32,
        start_x: f32,
        start_y: f32,
        direction: Direction,
        route: Route,
        waypoints: Vec<Waypoint>,
        sprite_index: u8,
    ) -> Self {
        let initial_angle = match direction {
            Direction::North => 180.0,
            Direction::South => 0.0,
            Direction::East => 270.0,
            Direction::West => 90.0,
        };

        Self {
            id,
            x: start_x,
            y: start_y,
            angle: initial_angle,
            speed: Speed::Medium,
            direction,
            route,
            waypoints,
            waypoint_index: 0,
            entry_time: Instant::now(),
            exit_time: None,
            active: true,
            in_intersection: false,
            reservation_id: None,
            sprite_index,
            anim: AnimState::new(initial_angle),
        }
    }

    /// Advance one simulation tick. Returns true when this vehicle is finished.
    pub fn advance(&mut self) -> bool {
        if self.waypoint_index >= self.waypoints.len() {
            if self.exit_time.is_none() {
                self.exit_time = Some(Instant::now());
            }
            self.active = false;
            return true;
        }

        let old_x = self.x;
        let old_y = self.y;
        let wp = &self.waypoints[self.waypoint_index];
        let dx = wp.x - self.x;
        let dy = wp.y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = self.speed.pixels_per_tick();

        if dist <= step {
            self.x = wp.x;
            self.y = wp.y;
            self.waypoint_index += 1;
        } else {
            self.x += (dx / dist) * step;
            self.y += (dy / dist) * step;
        }

        // Update heading after waypoint advancement so there is no one-frame
        // mismatch at segment/turn boundaries.
        let (hx, hy) = if self.waypoint_index < self.waypoints.len() {
            let next_wp = &self.waypoints[self.waypoint_index];
            (next_wp.x - self.x, next_wp.y - self.y)
        } else {
            (self.x - old_x, self.y - old_y)
        };

        if hx.abs() > f32::EPSILON || hy.abs() > f32::EPSILON {
            self.angle = hy.atan2(hx).to_degrees() + 90.0;
        }

        self.anim.tick(self.angle);
        false
    }

    pub fn predicted_path(&self) -> &[Waypoint] {
        if self.waypoint_index < self.waypoints.len() {
            &self.waypoints[self.waypoint_index..]
        } else {
            &[]
        }
    }

    pub fn distance_to(&self, other: &Vehicle) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn crossing_time(&self) -> Option<f32> {
        self.exit_time
            .map(|t| t.duration_since(self.entry_time).as_secs_f32())
    }
}

