// intersection.rs — Grid geometry, waypoint generation, and the
// "Reservation + Distance-based" smart management algorithm.
//
// Smart Intersection Logic (deadlock prevention):
// ─────────────────────────────────────────────────
// 1. RESERVATION: Before a vehicle enters the conflict zone it requests a
//    reservation.  The manager checks every OTHER vehicle's predicted path.
//    If any predicted waypoint of another vehicle is within CONFLICT_RADIUS of
//    any predicted waypoint of the requester, the reservation is DENIED and the
//    requesting vehicle decelerates to Speed::Slow until it can get a slot.
//
// 2. LOOK-AHEAD: Every tick, each vehicle checks the vehicle directly ahead of
//    it (same approach lane).  If the gap drops below SAFE_DISTANCE the
//    vehicle downshifts one speed level.  This prevents rear-end collisions in
//    the approach queues.
//
// 3. NO DEADLOCK: Because the conflict zone is cleared FIFO (first vehicle to
//    request while the zone is free gets it), and vehicles behind it slow down
//    via look-ahead, there is no circular dependency that could cause a
//    deadlock.

use crate::vehicle::{Direction, Route, Vehicle, Waypoint};

pub const WINDOW_W: i32 = 800;
pub const WINDOW_H: i32 = 800;

// Intersection box (the actual conflict zone in the center)
pub const ISECT_X1: f32 = 300.0;
pub const ISECT_Y1: f32 = 300.0;
pub const ISECT_X2: f32 = 500.0;
pub const ISECT_Y2: f32 = 500.0;
pub const ISECT_CX: f32 = (ISECT_X1 + ISECT_X2) / 2.0;
pub const ISECT_CY: f32 = (ISECT_Y1 + ISECT_Y2) / 2.0;

/// Lane offsets within the road (two lanes per road, each 40 px wide)
/// Road width = 80 px, centred on the window half-axis.
const ROAD_HALF: f32 = 40.0;
const LANE_HALF: f32 = 20.0;

/// Safe following distance in pixels
pub const SAFE_DISTANCE: f32 = 55.0;
/// Radius used when checking path overlap for reservations
const CONFLICT_RADIUS: f32 = 30.0;
/// How close two vehicles must get to count as a "close call"
pub const CLOSE_CALL_DIST: f32 = 40.0;

pub struct Intersection {
    pub next_reservation_id: u32,
}

impl Intersection {
    pub fn new() -> Self {
        Intersection { next_reservation_id: 1 }
    }

    // ─── Waypoint generation ──────────────────────────────────────────────

    /// Build the waypoint list for a vehicle given its entry direction and route.
    /// Waypoints are placed densely enough that smooth rotation looks good.
    pub fn build_waypoints(dir: Direction, route: Route) -> Vec<Waypoint> {
        // Lane centres on each approach road:
        //   North approach: x = CX - LANE_HALF  (right lane going south)
        //   South approach: x = CX + LANE_HALF  (right lane going north)
        //   East  approach: y = CY - LANE_HALF  (right lane going west)
        //   West  approach: y = CY + LANE_HALF  (right lane going east)
        match (dir, route) {
            (Direction::North, Route::Straight) => straight_points(
                ISECT_CX - LANE_HALF, -20.0,
                ISECT_CX - LANE_HALF, WINDOW_H as f32 + 20.0,
            ),
            (Direction::North, Route::Right) => curve_points(
                ISECT_CX - LANE_HALF, -20.0,
                ISECT_X1, ISECT_CY - LANE_HALF,
                true,
            ),
            (Direction::North, Route::Left) => curve_points(
                ISECT_CX - LANE_HALF, -20.0,
                ISECT_X2, ISECT_CY + LANE_HALF,
                false,
            ),

            (Direction::South, Route::Straight) => straight_points(
                ISECT_CX + LANE_HALF, WINDOW_H as f32 + 20.0,
                ISECT_CX + LANE_HALF, -20.0,
            ),
            (Direction::South, Route::Right) => curve_points(
                ISECT_CX + LANE_HALF, WINDOW_H as f32 + 20.0,
                ISECT_X2, ISECT_CY + LANE_HALF,
                true,
            ),
            (Direction::South, Route::Left) => curve_points(
                ISECT_CX + LANE_HALF, WINDOW_H as f32 + 20.0,
                ISECT_X1, ISECT_CY - LANE_HALF,
                false,
            ),

            (Direction::West, Route::Straight) => straight_points(
                -20.0, ISECT_CY + LANE_HALF,
                WINDOW_W as f32 + 20.0, ISECT_CY + LANE_HALF,
            ),
            (Direction::West, Route::Right) => curve_points(
                -20.0, ISECT_CY + LANE_HALF,
                ISECT_CX - LANE_HALF, ISECT_Y1,
                true,
            ),
            (Direction::West, Route::Left) => curve_points(
                -20.0, ISECT_CY + LANE_HALF,
                ISECT_CX + LANE_HALF, ISECT_Y2,
                false,
            ),

            (Direction::East, Route::Straight) => straight_points(
                WINDOW_W as f32 + 20.0, ISECT_CY - LANE_HALF,
                -20.0, ISECT_CY - LANE_HALF,
            ),
            (Direction::East, Route::Right) => curve_points(
                WINDOW_W as f32 + 20.0, ISECT_CY - LANE_HALF,
                ISECT_CX + LANE_HALF, ISECT_Y2,
                true,
            ),
            (Direction::East, Route::Left) => curve_points(
                WINDOW_W as f32 + 20.0, ISECT_CY - LANE_HALF,
                ISECT_CX - LANE_HALF, ISECT_Y1,
                false,
            ),
        }
    }

    /// Entry position for a vehicle spawning from a direction
    pub fn spawn_position(dir: Direction) -> (f32, f32) {
        match dir {
            Direction::North => (ISECT_CX - LANE_HALF, -20.0),
            Direction::South => (ISECT_CX + LANE_HALF, WINDOW_H as f32 + 20.0),
            Direction::West  => (-20.0, ISECT_CY + LANE_HALF),
            Direction::East  => (WINDOW_W as f32 + 20.0, ISECT_CY - LANE_HALF),
        }
    }

    // ─── Smart management: Reservation algorithm ─────────────────────────

    /// Try to grant a reservation to vehicle at index `req_idx`.
    /// Returns true if granted.
    /// The algorithm checks predicted path overlap against ALL vehicles that
    /// already hold a reservation (i.e. are in or about to enter the zone).
    pub fn try_reserve(&mut self, vehicles: &mut Vec<Vehicle>, req_idx: usize) -> bool {
        let req_path: Vec<Waypoint> = vehicles[req_idx]
            .predicted_path()
            .to_vec();

        for i in 0..vehicles.len() {
            if i == req_idx { continue; }
            if vehicles[i].reservation_id.is_none() { continue; }
            // Check every pair of waypoints for proximity
            for rw in &req_path {
                for ow in vehicles[i].predicted_path() {
                    let dx = rw.x - ow.x;
                    let dy = rw.y - ow.y;
                    if (dx * dx + dy * dy).sqrt() < CONFLICT_RADIUS {
                        return false; // path conflict detected → deny
                    }
                }
            }
        }
        // Grant reservation
        let rid = self.next_reservation_id;
        self.next_reservation_id += 1;
        vehicles[req_idx].reservation_id = Some(rid);
        true
    }

    /// Release a reservation when a vehicle exits the conflict zone
    pub fn release_reservation(vehicle: &mut Vehicle) {
        vehicle.reservation_id = None;
        vehicle.in_intersection = false;
    }

    /// Returns true if the vehicle's current position is inside the conflict box
    pub fn in_conflict_zone(v: &Vehicle) -> bool {
        v.x >= ISECT_X1 && v.x <= ISECT_X2 && v.y >= ISECT_Y1 && v.y <= ISECT_Y2
    }
}

// ─── Helper: straight line waypoints ─────────────────────────────────────────

fn straight_points(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Waypoint> {
    let steps = 60usize;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            Waypoint { x: x0 + (x1 - x0) * t, y: y0 + (y1 - y0) * t }
        })
        .collect()
}

/// Quadratic Bézier curve waypoints.
/// `clockwise` determines which side the control point bulges toward.
fn curve_points(x0: f32, y0: f32, x2: f32, y2: f32, clockwise: bool) -> Vec<Waypoint> {
    // Build a control point that creates a smooth 90° arc
    let cx_offset = if clockwise { x2 - x0 } else { 0.0 };
    let cy_offset = if clockwise { 0.0 } else { y2 - y0 };
    let cx1 = x0 + cx_offset;
    let cy1 = y0 + cy_offset;

    let steps = 60usize;
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let mt = 1.0 - t;
            // Quadratic Bézier: B(t) = (1-t)²P0 + 2(1-t)tP1 + t²P2
            let x = mt * mt * x0 + 2.0 * mt * t * cx1 + t * t * x2;
            let y = mt * mt * y0 + 2.0 * mt * t * cy1 + t * t * y2;
            Waypoint { x, y }
        })
        .collect()
}
