// intersection.rs - Intersection geometry, waypoint generation, and the
// reservation-based smart traffic algorithm.

use crate::vehicle::{Direction, Route, Vehicle, Waypoint};

pub const WINDOW_W: i32 = 800;
pub const WINDOW_H: i32 = 800;

// Intersection box (the conflict zone in the center)
pub const ISECT_X1: f32 = 300.0;
pub const ISECT_Y1: f32 = 300.0;
pub const ISECT_X2: f32 = 500.0;
pub const ISECT_Y2: f32 = 500.0;
pub const ISECT_CX: f32 = (ISECT_X1 + ISECT_X2) / 2.0;
pub const ISECT_CY: f32 = (ISECT_Y1 + ISECT_Y2) / 2.0;

// Three inbound lanes per approach: Left / Straight / Right.
const LANE_SPACING: f32 = 34.0;

/// Safe following distance in pixels.
pub const SAFE_DISTANCE: f32 = 55.0;
/// Radius used when checking path overlap for reservations.
const CONFLICT_RADIUS: f32 = 30.0;
/// How close two vehicles must get to count as a close call.
pub const CLOSE_CALL_DIST: f32 = SAFE_DISTANCE;

pub struct Intersection {
    pub next_reservation_id: u32,
}

impl Intersection {
    pub fn new() -> Self {
        Self {
            next_reservation_id: 1,
        }
    }

    /// Build waypoints for one vehicle from approach direction and route.
    /// Turning paths are curved and continue to the edge of the map.
    pub fn build_waypoints(dir: Direction, route: Route) -> Vec<Waypoint> {
        match (dir, route) {
            (Direction::North, Route::Straight) => straight_points(
                ns_lane_x(Route::Straight),
                -20.0,
                ns_lane_x(Route::Straight),
                WINDOW_H as f32 + 20.0,
            ),
            (Direction::North, Route::Right) => extend_with_straight(
                curve_points(
                    ns_lane_x(Route::Right),
                    -20.0,
                    ISECT_X1,
                    ISECT_CY - LANE_SPACING,
                    true,
                ),
                -20.0,
                ISECT_CY - LANE_SPACING,
            ),
            (Direction::North, Route::Left) => extend_with_straight(
                curve_points(
                    ns_lane_x(Route::Left),
                    -20.0,
                    ISECT_X2,
                    ISECT_CY - LANE_SPACING,
                    false,
                ),
                WINDOW_W as f32 + 20.0,
                ISECT_CY - LANE_SPACING,
            ),

            (Direction::South, Route::Straight) => straight_points(
                ns_lane_x(Route::Straight),
                WINDOW_H as f32 + 20.0,
                ns_lane_x(Route::Straight),
                -20.0,
            ),
            (Direction::South, Route::Right) => extend_with_straight(
                curve_points(
                    ns_lane_x(Route::Right),
                    WINDOW_H as f32 + 20.0,
                    ISECT_X2,
                    ISECT_CY + LANE_SPACING,
                    true,
                ),
                WINDOW_W as f32 + 20.0,
                ISECT_CY + LANE_SPACING,
            ),
            (Direction::South, Route::Left) => extend_with_straight(
                curve_points(
                    ns_lane_x(Route::Left),
                    WINDOW_H as f32 + 20.0,
                    ISECT_X1,
                    ISECT_CY + LANE_SPACING,
                    false,
                ),
                -20.0,
                ISECT_CY + LANE_SPACING,
            ),

            (Direction::West, Route::Straight) => straight_points(
                -20.0,
                west_lane_y(Route::Straight),
                WINDOW_W as f32 + 20.0,
                west_lane_y(Route::Straight),
            ),
            (Direction::West, Route::Right) => extend_with_straight(
                curve_points(
                    -20.0,
                    west_lane_y(Route::Right),
                    ISECT_CX + LANE_SPACING,
                    ISECT_Y2,
                    true,
                ),
                ISECT_CX + LANE_SPACING,
                WINDOW_H as f32 + 20.0,
            ),
            (Direction::West, Route::Left) => extend_with_straight(
                curve_points(
                    -20.0,
                    west_lane_y(Route::Left),
                    ISECT_CX - LANE_SPACING,
                    ISECT_Y1,
                    false,
                ),
                ISECT_CX - LANE_SPACING,
                -20.0,
            ),

            (Direction::East, Route::Straight) => straight_points(
                WINDOW_W as f32 + 20.0,
                east_lane_y(Route::Straight),
                -20.0,
                east_lane_y(Route::Straight),
            ),
            (Direction::East, Route::Right) => extend_with_straight(
                curve_points(
                    WINDOW_W as f32 + 20.0,
                    east_lane_y(Route::Right),
                    ISECT_CX - LANE_SPACING,
                    ISECT_Y1,
                    true,
                ),
                ISECT_CX - LANE_SPACING,
                -20.0,
            ),
            (Direction::East, Route::Left) => extend_with_straight(
                curve_points(
                    WINDOW_W as f32 + 20.0,
                    east_lane_y(Route::Left),
                    ISECT_CX + LANE_SPACING,
                    ISECT_Y2,
                    false,
                ),
                ISECT_CX + LANE_SPACING,
                WINDOW_H as f32 + 20.0,
            ),
        }
    }

    /// Entry position for a vehicle spawning from a direction + route lane.
    pub fn spawn_position(dir: Direction, route: Route) -> (f32, f32) {
        match dir {
            Direction::North => (ns_lane_x(route), -20.0),
            Direction::South => (ns_lane_x(route), WINDOW_H as f32 + 20.0),
            Direction::West => (-20.0, west_lane_y(route)),
            Direction::East => (WINDOW_W as f32 + 20.0, east_lane_y(route)),
        }
    }

    /// Try to grant a reservation to vehicle at index `req_idx`.
    /// Returns true if granted.
    pub fn try_reserve(&mut self, vehicles: &mut Vec<Vehicle>, req_idx: usize) -> bool {
        let req_path: Vec<Waypoint> = vehicles[req_idx].predicted_path().to_vec();

        for i in 0..vehicles.len() {
            if i == req_idx {
                continue;
            }
            if vehicles[i].reservation_id.is_none() {
                continue;
            }

            for rw in &req_path {
                for ow in vehicles[i].predicted_path() {
                    let dx = rw.x - ow.x;
                    let dy = rw.y - ow.y;
                    if (dx * dx + dy * dy).sqrt() < CONFLICT_RADIUS {
                        return false;
                    }
                }
            }
        }

        let rid = self.next_reservation_id;
        self.next_reservation_id += 1;
        vehicles[req_idx].reservation_id = Some(rid);
        true
    }

    /// Release a reservation when a vehicle exits the conflict zone.
    pub fn release_reservation(vehicle: &mut Vehicle) {
        vehicle.reservation_id = None;
        vehicle.in_intersection = false;
    }

    /// Returns true if the vehicle's current position is inside the conflict box.
    pub fn in_conflict_zone(v: &Vehicle) -> bool {
        v.x >= ISECT_X1 && v.x <= ISECT_X2 && v.y >= ISECT_Y1 && v.y <= ISECT_Y2
    }
}

/// Lane id on the inbound side for spacing/following checks.
pub fn incoming_lane_id(direction: Direction, route: Route) -> i8 {
    match direction {
        Direction::East => match route {
            Route::Right => 0,
            Route::Straight => 1,
            Route::Left => 2,
        },
        _ => match route {
            Route::Left => 0,
            Route::Straight => 1,
            Route::Right => 2,
        },
    }
}

fn ns_lane_x(route: Route) -> f32 {
    match route {
        Route::Left => ISECT_CX - LANE_SPACING,
        Route::Straight => ISECT_CX,
        Route::Right => ISECT_CX + LANE_SPACING,
    }
}

fn west_lane_y(route: Route) -> f32 {
    match route {
        Route::Left => ISECT_CY - LANE_SPACING,
        Route::Straight => ISECT_CY,
        Route::Right => ISECT_CY + LANE_SPACING,
    }
}

fn east_lane_y(route: Route) -> f32 {
    match route {
        Route::Right => ISECT_CY - LANE_SPACING,
        Route::Straight => ISECT_CY,
        Route::Left => ISECT_CY + LANE_SPACING,
    }
}

fn straight_points(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Waypoint> {
    let steps = 60usize;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            Waypoint {
                x: x0 + (x1 - x0) * t,
                y: y0 + (y1 - y0) * t,
            }
        })
        .collect()
}

fn extend_with_straight(mut path: Vec<Waypoint>, x1: f32, y1: f32) -> Vec<Waypoint> {
    let Some(last) = path.last().cloned() else {
        return path;
    };

    let tail = straight_points(last.x, last.y, x1, y1);
    path.extend(tail.into_iter().skip(1));
    path
}

/// Quadratic Bezier curve waypoints.
/// `clockwise` determines which side the control point bulges toward.
fn curve_points(x0: f32, y0: f32, x2: f32, y2: f32, clockwise: bool) -> Vec<Waypoint> {
    let cx_offset = if clockwise { x2 - x0 } else { 0.0 };
    let cy_offset = if clockwise { 0.0 } else { y2 - y0 };
    let cx1 = x0 + cx_offset;
    let cy1 = y0 + cy_offset;

    let steps = 60usize;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            let x = mt * mt * x0 + 2.0 * mt * t * cx1 + t * t * x2;
            let y = mt * mt * y0 + 2.0 * mt * t * cy1 + t * t * y2;
            Waypoint { x, y }
        })
        .collect()
}
