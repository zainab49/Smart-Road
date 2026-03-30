// vehicle.rs — Vehicle struct, kinematics, and routing logic

use std::time::Instant;

/// Cardinal direction a vehicle enters/exits from
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    North,
    South,
    East,
    West,
}

/// The intended maneuver through the intersection
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    Left,
    Straight,
    Right,
}

/// Discrete speed levels (v = d/t satisfied by fixed step sizes)
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Speed {
    Slow   = 1,
    Normal = 2,
    Fast   = 3,
}

impl Speed {
    pub fn pixels_per_tick(self) -> f32 {
        match self {
            Speed::Slow   => 1.5,
            Speed::Normal => 2.5,
            Speed::Fast   => 4.0,
        }
    }

    pub fn downshift(self) -> Speed {
        match self {
            Speed::Fast   => Speed::Normal,
            Speed::Normal => Speed::Slow,
            Speed::Slow   => Speed::Slow,
        }
    }
}

/// Waypoint along the curved / straight path through the intersection
#[derive(Clone, Debug)]
pub struct Waypoint {
    pub x: f32,
    pub y: f32,
}

/// One autonomous vehicle
pub struct Vehicle {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub angle: f32,           // degrees, for smooth rotation rendering
    pub speed: Speed,
    pub direction: Direction, // entry direction
    pub route: Route,
    pub waypoints: Vec<Waypoint>,
    pub waypoint_index: usize,
    pub entry_time: Instant,
    pub exit_time: Option<Instant>,
    pub active: bool,
    pub in_intersection: bool,
    /// reservation slot granted by the intersection manager
    pub reservation_id: Option<u32>,
    pub color_r: u8,
    pub color_g: u8,
    pub color_b: u8,
}

impl Vehicle {
    pub fn new(
        id: u32,
        start_x: f32,
        start_y: f32,
        direction: Direction,
        route: Route,
        waypoints: Vec<Waypoint>,
        color: (u8, u8, u8),
    ) -> Self {
        let initial_angle = match direction {
            Direction::North => 180.0,
            Direction::South => 0.0,
            Direction::East  => 270.0,
            Direction::West  => 90.0,
        };
        Vehicle {
            id,
            x: start_x,
            y: start_y,
            angle: initial_angle,
            speed: Speed::Normal,
            direction,
            route,
            waypoints,
            waypoint_index: 0,
            entry_time: Instant::now(),
            exit_time: None,
            active: true,
            in_intersection: false,
            reservation_id: None,
            color_r: color.0,
            color_g: color.1,
            color_b: color.2,
        }
    }

    /// Advance vehicle toward next waypoint. Returns true when the vehicle
    /// has consumed all waypoints and should be removed.
    pub fn advance(&mut self) -> bool {
        if self.waypoint_index >= self.waypoints.len() {
            if self.exit_time.is_none() {
                self.exit_time = Some(Instant::now());
            }
            self.active = false;
            return true;
        }

        let wp = &self.waypoints[self.waypoint_index];
        let dx = wp.x - self.x;
        let dy = wp.y - self.y;
        let dist = (dx * dx + dy * dy).sqrt();
        let step = self.speed.pixels_per_tick();

        // Smoothly rotate toward waypoint direction
        let target_angle = dy.atan2(dx).to_degrees() + 90.0;
        let angle_diff = normalize_angle(target_angle - self.angle);
        self.angle += angle_diff * 0.15; // lerp factor for smooth rotation

        if dist <= step {
            self.x = wp.x;
            self.y = wp.y;
            self.waypoint_index += 1;
        } else {
            self.x += (dx / dist) * step;
            self.y += (dy / dist) * step;
        }
        false
    }

    /// Remaining waypoints as predicted path (for reservation checks)
    pub fn predicted_path(&self) -> &[Waypoint] {
        if self.waypoint_index < self.waypoints.len() {
            &self.waypoints[self.waypoint_index..]
        } else {
            &[]
        }
    }

    /// Euclidean distance to another vehicle
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

fn normalize_angle(a: f32) -> f32 {
    let mut a = a % 360.0;
    if a > 180.0  { a -= 360.0; }
    if a < -180.0 { a += 360.0; }
    a
}
