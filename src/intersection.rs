// intersection.rs - Intersection geometry, waypoint generation, and the
// reservation-based smart traffic algorithm.

use std::collections::{HashMap, HashSet};

use crate::vehicle::{Direction, Route, Vehicle, Waypoint};

pub const WINDOW_W: i32 = 800;
pub const WINDOW_H: i32 = 800;

// Intersection box (the visual center conflict box).
pub const ISECT_X1: f32 = 300.0;
pub const ISECT_Y1: f32 = 300.0;
pub const ISECT_X2: f32 = 500.0;
pub const ISECT_Y2: f32 = 500.0;
pub const ISECT_CX: f32 = (ISECT_X1 + ISECT_X2) / 2.0;
pub const ISECT_CY: f32 = (ISECT_Y1 + ISECT_Y2) / 2.0;

// Grid-reservation area.
pub const GRID_X1: f32 = 240.0;
pub const GRID_Y1: f32 = 240.0;
pub const GRID_X2: f32 = 560.0;
pub const GRID_Y2: f32 = 560.0;

// 4-way intersection lane model:
// each road has 6 lanes total = 3 inbound + 3 outbound.
pub const LANE_SPACING: f32 = 34.0;
pub const GUIDE_LINE_SPACING: f32 = LANE_SPACING * 1.45;
const LANES_PER_DIRECTION: usize = 3;
const ROAD_HALF_WIDTH: f32 = GUIDE_LINE_SPACING * LANES_PER_DIRECTION as f32;

/// Safe following distance in pixels.
pub const SAFE_DISTANCE: f32 = 55.0;
/// Radius used when checking path overlap for reservations.
const CONFLICT_RADIUS: f32 = 30.0;
/// Number of cells per side in the conflict-zone grid.
pub const CONFLICT_GRID_DIVS: usize = 6;
/// How close two vehicles must get to count as a close call.
pub const CLOSE_CALL_DIST: f32 = SAFE_DISTANCE;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ConflictCell {
    row: u8,
    col: u8,
}

pub struct Intersection {
    pub next_reservation_id: u32,
    reserved_cells: HashMap<u32, HashSet<ConflictCell>>,
}

impl Intersection {
    pub fn new() -> Self {
        Self {
            next_reservation_id: 1,
            reserved_cells: HashMap::new(),
        }
    }

    /// Build waypoints for one vehicle from approach direction and route.
    /// Inbound lane usage is dedicated:
    /// - right lane: right turn
    /// - middle lane: straight
    /// - left lane: left turn
    pub fn build_waypoints(dir: Direction, route: Route) -> Vec<Waypoint> {
        match (dir, route) {
            (Direction::North, Route::Straight) => straight_points(
                north_inbound_x(Route::Straight),
                -20.0,
                south_outbound_x(Route::Straight),
                WINDOW_H as f32 + 20.0,
            ),
            (Direction::North, Route::Right) => extend_with_straight(
                right_angle_points(
                    north_inbound_x(Route::Right),
                    -20.0,
                    north_inbound_x(Route::Right),
                    west_outbound_y(Route::Right),
                    north_inbound_x(Route::Right) - GUIDE_LINE_SPACING,
                    west_outbound_y(Route::Right),
                ),
                -20.0,
                west_outbound_y(Route::Right),
            ),
            (Direction::North, Route::Left) => extend_with_straight(
                right_angle_points(
                    north_inbound_x(Route::Left),
                    -20.0,
                    north_inbound_x(Route::Left),
                    east_outbound_y(Route::Left),
                    ISECT_X2,
                    east_outbound_y(Route::Left),
                ),
                WINDOW_W as f32 + 20.0,
                east_outbound_y(Route::Left),
            ),

            (Direction::South, Route::Straight) => straight_points(
                south_inbound_x(Route::Straight),
                WINDOW_H as f32 + 20.0,
                north_outbound_x(Route::Straight),
                -20.0,
            ),
            (Direction::South, Route::Right) => extend_with_straight(
                right_angle_points(
                    south_inbound_x(Route::Right),
                    WINDOW_H as f32 + 20.0,
                    south_inbound_x(Route::Right),
                    east_outbound_y(Route::Right),
                    south_inbound_x(Route::Right) + GUIDE_LINE_SPACING,
                    east_outbound_y(Route::Right),
                ),
                WINDOW_W as f32 + 20.0,
                east_outbound_y(Route::Right),
            ),
            (Direction::South, Route::Left) => extend_with_straight(
                right_angle_points(
                    south_inbound_x(Route::Left),
                    WINDOW_H as f32 + 20.0,
                    south_inbound_x(Route::Left),
                    west_outbound_y(Route::Left),
                    ISECT_X1,
                    west_outbound_y(Route::Left),
                ),
                -20.0,
                west_outbound_y(Route::Left),
            ),

            (Direction::West, Route::Straight) => straight_points(
                -20.0,
                west_inbound_y(Route::Straight),
                WINDOW_W as f32 + 20.0,
                east_outbound_y(Route::Straight),
            ),
            (Direction::West, Route::Right) => extend_with_straight(
                right_angle_points(
                    -20.0,
                    west_inbound_y(Route::Right),
                    south_outbound_x(Route::Right),
                    west_inbound_y(Route::Right),
                    south_outbound_x(Route::Right),
                    west_inbound_y(Route::Right) + GUIDE_LINE_SPACING,
                ),
                south_outbound_x(Route::Right),
                WINDOW_H as f32 + 20.0,
            ),
            (Direction::West, Route::Left) => extend_with_straight(
                right_angle_points(
                    -20.0,
                    west_inbound_y(Route::Left),
                    north_outbound_x(Route::Left),
                    west_inbound_y(Route::Left),
                    north_outbound_x(Route::Left),
                    ISECT_Y1,
                ),
                north_outbound_x(Route::Left),
                -20.0,
            ),

            (Direction::East, Route::Straight) => straight_points(
                WINDOW_W as f32 + 20.0,
                east_inbound_y(Route::Straight),
                -20.0,
                west_outbound_y(Route::Straight),
            ),
            (Direction::East, Route::Right) => extend_with_straight(
                right_angle_points(
                    WINDOW_W as f32 + 20.0,
                    east_inbound_y(Route::Right),
                    north_outbound_x(Route::Right),
                    east_inbound_y(Route::Right),
                    north_outbound_x(Route::Right),
                    east_inbound_y(Route::Right) - GUIDE_LINE_SPACING,
                ),
                north_outbound_x(Route::Right),
                -20.0,
            ),
            (Direction::East, Route::Left) => extend_with_straight(
                right_angle_points(
                    WINDOW_W as f32 + 20.0,
                    east_inbound_y(Route::Left),
                    south_outbound_x(Route::Left),
                    east_inbound_y(Route::Left),
                    south_outbound_x(Route::Left),
                    ISECT_Y2,
                ),
                south_outbound_x(Route::Left),
                WINDOW_H as f32 + 20.0,
            ),
        }
    }

    /// Entry position for a vehicle spawning from a direction + route lane.
    pub fn spawn_position(dir: Direction, route: Route) -> (f32, f32) {
        match dir {
            Direction::North => (north_inbound_x(route), -20.0),
            Direction::South => (south_inbound_x(route), WINDOW_H as f32 + 20.0),
            Direction::West => (-20.0, west_inbound_y(route)),
            Direction::East => (WINDOW_W as f32 + 20.0, east_inbound_y(route)),
        }
    }

    /// Try to grant a reservation to vehicle at index `req_idx`.
    /// Returns true if granted.
    pub fn try_reserve(&mut self, vehicles: &mut [Vehicle], req_idx: usize) -> bool {
        let req_cells = Self::predicted_conflict_cells(&vehicles[req_idx]);

        for cells in self.reserved_cells.values() {
            if req_cells.iter().any(|c| cells.contains(c)) {
                return false;
            }
        }

        // Fallback safety check against already reserved vehicles in case
        // two paths pass very close but map to neighboring cells.
        let req_path = vehicles[req_idx].predicted_path().to_vec();
        for i in 0..vehicles.len() {
            if i == req_idx || vehicles[i].reservation_id.is_none() {
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
        self.reserved_cells.insert(rid, req_cells);
        true
    }

    /// Release a reservation when a vehicle exits the conflict zone.
    pub fn release_reservation(&mut self, vehicle: &mut Vehicle) {
        if let Some(rid) = vehicle.reservation_id.take() {
            self.reserved_cells.remove(&rid);
        }
        vehicle.in_intersection = false;
    }

    /// Returns true if the vehicle's current position is inside the conflict box.
    pub fn in_conflict_zone(v: &Vehicle) -> bool {
        v.x >= GRID_X1 && v.x <= GRID_X2 && v.y >= GRID_Y1 && v.y <= GRID_Y2
    }

    fn predicted_conflict_cells(vehicle: &Vehicle) -> HashSet<ConflictCell> {
        let mut cells = HashSet::new();

        if let Some(cell) = Self::conflict_cell_at(vehicle.x, vehicle.y) {
            cells.insert(cell);
        }

        for wp in vehicle.predicted_path() {
            if let Some(cell) = Self::conflict_cell_at(wp.x, wp.y) {
                cells.insert(cell);
            }
        }

        cells
    }

    fn conflict_cell_at(x: f32, y: f32) -> Option<ConflictCell> {
        if x < GRID_X1 || x > GRID_X2 || y < GRID_Y1 || y > GRID_Y2 {
            return None;
        }

        let grid = CONFLICT_GRID_DIVS as f32;
        let col = (((x - GRID_X1) / (GRID_X2 - GRID_X1)) * grid).floor() as i32;
        let row = (((y - GRID_Y1) / (GRID_Y2 - GRID_Y1)) * grid).floor() as i32;

        let col = col.clamp(0, CONFLICT_GRID_DIVS as i32 - 1) as u8;
        let row = row.clamp(0, CONFLICT_GRID_DIVS as i32 - 1) as u8;
        Some(ConflictCell { row, col })
    }
}

/// Lane id on the inbound side for spacing/following checks.
pub fn incoming_lane_id(direction: Direction, route: Route) -> i8 {
    // Create unique lane IDs: direction * 3 + route 
    // This ensures each direction/route combo gets a unique lane ID
    let dir_offset = match direction {
        Direction::North => 0,
        Direction::South => 3,
        Direction::East => 6, 
        Direction::West => 9,
    };
    dir_offset + route_lane_idx(route) as i8
}

fn route_lane_idx(route: Route) -> usize {
    match route {
        Route::Right => 0,
        Route::Straight => 1,
        Route::Left => 2,
    }
}

fn lane_offset(lane_idx: usize) -> f32 {
    ROAD_HALF_WIDTH - (lane_idx as f32 + 0.5) * GUIDE_LINE_SPACING
}

fn north_inbound_x(route: Route) -> f32 {
    ISECT_CX - lane_offset(route_lane_idx(route))
}

fn south_inbound_x(route: Route) -> f32 {
    ISECT_CX + lane_offset(route_lane_idx(route))
}

fn west_inbound_y(route: Route) -> f32 {
    ISECT_CY + lane_offset(route_lane_idx(route))
}

fn east_inbound_y(route: Route) -> f32 {
    ISECT_CY - lane_offset(route_lane_idx(route))
}

fn north_outbound_x(route: Route) -> f32 {
    ISECT_CX + lane_offset(route_lane_idx(route))
}

fn south_outbound_x(route: Route) -> f32 {
    ISECT_CX - lane_offset(route_lane_idx(route))
}

fn west_outbound_y(route: Route) -> f32 {
    ISECT_CY - lane_offset(route_lane_idx(route))
}

fn east_outbound_y(route: Route) -> f32 {
    ISECT_CY + lane_offset(route_lane_idx(route))
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

fn right_angle_points(
    x0: f32,
    y0: f32,
    corner_x: f32,
    corner_y: f32,
    x1: f32,
    y1: f32,
) -> Vec<Waypoint> {
    let mut path = straight_points(x0, y0, corner_x, corner_y);
    let leg2 = straight_points(corner_x, corner_y, x1, y1);
    path.extend(leg2.into_iter().skip(1));
    path
}
