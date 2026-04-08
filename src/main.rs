// main.rs - Smart road 4-way intersection simulation without traffic lights.

mod animation;
mod intersection;
mod renderer;
mod stats;
mod vehicle;

use std::collections::HashSet;
use std::time::{Duration, Instant};

use rand::Rng;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use intersection::{
    incoming_lane_id, Intersection, CLOSE_CALL_DIST, SAFE_DISTANCE, WINDOW_H, WINDOW_W,
};
use renderer::{draw_simulation, draw_stats_screen, GameTextures};
use stats::Stats;
use vehicle::{Direction, Route, Speed, Vehicle};

const TARGET_FPS: u64 = 60;
const FRAME_MS: u64 = 1000 / TARGET_FPS;
const AUTO_SPAWN_INTERVAL_TICKS: u32 = 16;

fn main() {
    let sdl = sdl2::init().expect("SDL2 init");
    let video = sdl.video().expect("SDL2 video");

    let window = video
        .window(
            "Smart Road Intersection | Arrows=spawn random route, R=toggle random, ESC=stats",
            WINDOW_W as u32,
            WINDOW_H as u32,
        )
        .position_centered()
        .build()
        .expect("Window");

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .expect("Canvas");

    let texture_creator = canvas.texture_creator();
    let textures = GameTextures::load(&texture_creator);
    let mut events = sdl.event_pump().expect("Event pump");
    let mut rng = rand::thread_rng();

    let mut intersection = Intersection::new();
    let mut stats = Stats::new();
    let mut vehicles: Vec<Vehicle> = Vec::new();
    let mut active_close_pairs: HashSet<(u32, u32)> = HashSet::new();

    let mut next_vehicle_id: u32 = 1;
    let mut random_spawn_enabled = false;
    let mut auto_spawn_tick = 0u32;

    'simulation: loop {
        let frame_start = Instant::now();

        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'simulation,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'simulation,
                Event::KeyDown {
                    keycode: Some(Keycode::R),
                    repeat: false,
                    ..
                } => {
                    random_spawn_enabled = !random_spawn_enabled;
                    auto_spawn_tick = 0;
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Up),
                    repeat: false,
                    ..
                } => {
                    let route = random_route(&mut rng);
                    let _ = spawn_vehicle(
                        &mut vehicles,
                        &mut next_vehicle_id,
                        &mut rng,
                        Direction::South,
                        route,
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Down),
                    repeat: false,
                    ..
                } => {
                    let route = random_route(&mut rng);
                    let _ = spawn_vehicle(
                        &mut vehicles,
                        &mut next_vehicle_id,
                        &mut rng,
                        Direction::North,
                        route,
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Right),
                    repeat: false,
                    ..
                } => {
                    let route = random_route(&mut rng);
                    let _ = spawn_vehicle(
                        &mut vehicles,
                        &mut next_vehicle_id,
                        &mut rng,
                        Direction::West,
                        route,
                    );
                }
                Event::KeyDown {
                    keycode: Some(Keycode::Left),
                    repeat: false,
                    ..
                } => {
                    let route = random_route(&mut rng);
                    let _ = spawn_vehicle(
                        &mut vehicles,
                        &mut next_vehicle_id,
                        &mut rng,
                        Direction::East,
                        route,
                    );
                }
                _ => {}
            }
        }

        if random_spawn_enabled {
            auto_spawn_tick += 1;
            if auto_spawn_tick >= AUTO_SPAWN_INTERVAL_TICKS {
                auto_spawn_tick = 0;
                let direction = random_direction(&mut rng);
                let route = random_route(&mut rng);
                let _ = spawn_vehicle(
                    &mut vehicles,
                    &mut next_vehicle_id,
                    &mut rng,
                    direction,
                    route,
                );
            }
        }

        apply_following_logic(&mut vehicles);
        update_reservations_and_move(&mut vehicles, &mut intersection, &mut stats);
        update_close_calls(&vehicles, &mut active_close_pairs, &mut stats);

        draw_simulation(
            &mut canvas,
            &vehicles,
            &textures,
            WINDOW_W as u32,
            WINDOW_H as u32,
        );
        canvas.present();

        let elapsed = frame_start.elapsed();
        let target = Duration::from_millis(FRAME_MS);
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }

    for line in stats.report_lines() {
        println!("{line}");
    }

    let _ = canvas.window_mut().set_title(&stats.summary_title());

    'stats_view: loop {
        let frame_start = Instant::now();
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'stats_view,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'stats_view,
                _ => {}
            }
        }

        draw_stats_screen(&mut canvas, &stats, WINDOW_W as u32, WINDOW_H as u32);
        canvas.present();

        let elapsed = frame_start.elapsed();
        let target = Duration::from_millis(FRAME_MS);
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }
}

fn spawn_vehicle(
    vehicles: &mut Vec<Vehicle>,
    next_vehicle_id: &mut u32,
    rng: &mut rand::rngs::ThreadRng,
    direction: Direction,
    route: Route,
) -> bool {
    let (x, y) = Intersection::spawn_position(direction, route);
    if spawn_blocked(vehicles, direction, route, x, y) {
        return false;
    }

    let sprite_index = rng.gen_range(0..7) as u8;
    let waypoints = Intersection::build_waypoints(direction, route);
    let mut vehicle = Vehicle::new(
        *next_vehicle_id,
        x,
        y,
        direction,
        route,
        waypoints,
        sprite_index,
    );
    vehicle.speed = random_speed(rng);

    vehicles.push(vehicle);
    *next_vehicle_id += 1;
    true
}

fn spawn_blocked(vehicles: &[Vehicle], direction: Direction, route: Route, x: f32, y: f32) -> bool {
    let lane = incoming_lane_id(direction, route);
    vehicles.iter().any(|v| {
        if v.direction != direction || incoming_lane_id(v.direction, v.route) != lane {
            return false;
        }
        let dx = v.x - x;
        let dy = v.y - y;
        (dx * dx + dy * dy).sqrt() < SAFE_DISTANCE * 1.2
    })
}

fn random_direction(rng: &mut rand::rngs::ThreadRng) -> Direction {
    match rng.gen_range(0..4) {
        0 => Direction::North,
        1 => Direction::South,
        2 => Direction::East,
        _ => Direction::West,
    }
}

fn random_route(rng: &mut rand::rngs::ThreadRng) -> Route {
    match rng.gen_range(0..3) {
        0 => Route::Right,
        1 => Route::Straight,
        _ => Route::Left,
    }
}

fn random_speed(rng: &mut rand::rngs::ThreadRng) -> Speed {
    match rng.gen_range(0..3) {
        0 => Speed::Slow,
        1 => Speed::Medium,
        _ => Speed::Fast,
    }
}

fn apply_following_logic(vehicles: &mut [Vehicle]) {
    for i in 0..vehicles.len() {
        let mut nearest_ahead = f32::INFINITY;

        for j in 0..vehicles.len() {
            if i == j
                || vehicles[i].direction != vehicles[j].direction
                || incoming_lane_id(vehicles[i].direction, vehicles[i].route)
                    != incoming_lane_id(vehicles[j].direction, vehicles[j].route)
            {
                continue;
            }
            if !is_ahead(&vehicles[i], &vehicles[j]) {
                continue;
            }

            let gap = lane_gap(&vehicles[i], &vehicles[j]);
            if gap < nearest_ahead {
                nearest_ahead = gap;
            }
        }

        let target_speed = if nearest_ahead < SAFE_DISTANCE {
            Speed::Slow
        } else if nearest_ahead < SAFE_DISTANCE * 1.8 {
            Speed::Medium
        } else {
            Speed::Fast
        };

        // Left turns are tighter, so cap free-flow speed at medium.
        let route_limited = if vehicles[i].route == Route::Left && target_speed == Speed::Fast {
            Speed::Medium
        } else {
            target_speed
        };

        if vehicles[i].speed < route_limited {
            vehicles[i].speed = vehicles[i].speed.upshift();
        } else if vehicles[i].speed > route_limited {
            vehicles[i].speed = vehicles[i].speed.downshift();
        }
    }
}

fn is_ahead(self_v: &Vehicle, other: &Vehicle) -> bool {
    match self_v.direction {
        Direction::North => other.y > self_v.y,
        Direction::South => other.y < self_v.y,
        Direction::East => other.x < self_v.x,
        Direction::West => other.x > self_v.x,
    }
}

fn lane_gap(self_v: &Vehicle, other: &Vehicle) -> f32 {
    match self_v.direction {
        Direction::North | Direction::South => (other.y - self_v.y).abs(),
        Direction::East | Direction::West => (other.x - self_v.x).abs(),
    }
}

fn update_reservations_and_move(
    vehicles: &mut Vec<Vehicle>,
    intersection: &mut Intersection,
    stats: &mut Stats,
) {
    let mut wait_for_slot = vec![false; vehicles.len()];

    for i in 0..vehicles.len() {
        if Intersection::in_conflict_zone(&vehicles[i]) {
            vehicles[i].in_intersection = true;
            continue;
        }
        if vehicles[i].reservation_id.is_some() {
            continue;
        }
        if !near_conflict_zone(&vehicles[i]) {
            continue;
        }

        let granted = intersection.try_reserve(vehicles, i);
        if !granted {
            vehicles[i].speed = Speed::Slow;
            if will_enter_conflict_zone(&vehicles[i]) {
                wait_for_slot[i] = true;
            }
        }
    }

    let mut remove_indices: Vec<usize> = Vec::new();

    for i in 0..vehicles.len() {
        if wait_for_slot[i] {
            stats.observe_velocity(0.0);
            continue;
        }

        let was_in_zone = Intersection::in_conflict_zone(&vehicles[i]);
        let finished = vehicles[i].advance();
        let now_in_zone = Intersection::in_conflict_zone(&vehicles[i]);

        if !was_in_zone && now_in_zone {
            vehicles[i].in_intersection = true;
        }

        if was_in_zone && !now_in_zone && vehicles[i].reservation_id.is_some() {
            Intersection::release_reservation(&mut vehicles[i]);
        }

        let velocity_px_s = vehicles[i].speed.pixels_per_tick() * TARGET_FPS as f32;
        stats.observe_velocity(velocity_px_s);

        if finished {
            if let Some(crossing_time) = vehicles[i].crossing_time() {
                stats.record_passed_vehicle(crossing_time);
            }
            remove_indices.push(i);
        }
    }

    for idx in remove_indices.into_iter().rev() {
        vehicles.remove(idx);
    }
}

fn near_conflict_zone(v: &Vehicle) -> bool {
    // Request reservation before the center box to avoid last-second braking.
    let margin = 90.0;
    let min_x = intersection::ISECT_X1 - margin;
    let max_x = intersection::ISECT_X2 + margin;
    let min_y = intersection::ISECT_Y1 - margin;
    let max_y = intersection::ISECT_Y2 + margin;
    v.x >= min_x && v.x <= max_x && v.y >= min_y && v.y <= max_y
}

fn will_enter_conflict_zone(v: &Vehicle) -> bool {
    if v.waypoint_index >= v.waypoints.len() {
        return false;
    }

    let wp = &v.waypoints[v.waypoint_index];
    let dx = wp.x - v.x;
    let dy = wp.y - v.y;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist <= f32::EPSILON {
        return false;
    }

    let step = v.speed.pixels_per_tick().min(dist);
    let nx = v.x + dx / dist * step;
    let ny = v.y + dy / dist * step;

    nx >= intersection::ISECT_X1
        && nx <= intersection::ISECT_X2
        && ny >= intersection::ISECT_Y1
        && ny <= intersection::ISECT_Y2
}

fn update_close_calls(
    vehicles: &[Vehicle],
    active_pairs: &mut HashSet<(u32, u32)>,
    stats: &mut Stats,
) {
    let mut current_pairs: HashSet<(u32, u32)> = HashSet::new();

    for i in 0..vehicles.len() {
        for j in (i + 1)..vehicles.len() {
            if vehicles[i].distance_to(&vehicles[j]) < CLOSE_CALL_DIST {
                let pair = ordered_pair(vehicles[i].id, vehicles[j].id);
                current_pairs.insert(pair);
                if !active_pairs.contains(&pair) {
                    stats.record_close_call();
                }
            }
        }
    }

    *active_pairs = current_pairs;
}

fn ordered_pair(a: u32, b: u32) -> (u32, u32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}
