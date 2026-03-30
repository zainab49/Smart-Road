// main.rs — Entry point, game loop, input handling
//
// Controls:
//   Arrow Up    → Spawn vehicle from North
//   Arrow Down  → Spawn vehicle from South
//   Arrow Left  → Spawn vehicle from West
//   Arrow Right → Spawn vehicle from East
//   R           → Toggle random-spawn mode
//   Esc         → End simulation, show Statistics screen

mod vehicle;
mod intersection;
mod stats;
mod renderer;

use std::time::{Duration, Instant};

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use rand::Rng;

use vehicle::{Direction, Route, Speed, Vehicle};
use intersection::{Intersection, SAFE_DISTANCE, CLOSE_CALL_DIST, WINDOW_W, WINDOW_H};
use stats::Stats;
use renderer::{draw_world, draw_stats_screen};

// ─── Configuration ────────────────────────────────────────────────────────────

const TARGET_FPS: u64    = 60;
const FRAME_MS:   u64    = 1000 / TARGET_FPS;
/// Minimum ticks between two vehicles spawning from the same direction
const SPAWN_COOLDOWN_TICKS: u64 = 90;
/// Ticks between random spawns when R-mode is active
const RANDOM_SPAWN_INTERVAL: u64 = 60;

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let sdl_context = sdl2::init().expect("SDL2 init failed");
    let video      = sdl_context.video().expect("SDL2 video failed");

    let window = video
        .window("Smart Road — AV Intersection Simulation", WINDOW_W as u32, WINDOW_H as u32)
        .position_centered()
        .build()
        .expect("Window creation failed");

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .expect("Canvas creation failed");

    let mut event_pump = sdl_context.event_pump().expect("Event pump failed");

    let mut intersection  = Intersection::new();
    let mut vehicles: Vec<Vehicle> = Vec::new();
    let mut stats          = Stats::new();
    let mut next_id: u32   = 1;
    let mut random_mode    = false;
    let mut tick: u64      = 0;

    // Per-direction spawn cooldown (last tick a vehicle was spawned)
    let mut last_spawn = [0u64; 4]; // [N, S, W, E]
    let mut random_spawn_timer: u64 = 0;

    // ─── Simulation colours ──────────────────────────────────────────────
    let palette: &[(u8, u8, u8)] = &[
        (220,  60,  60), // red
        ( 60, 180, 220), // cyan
        (220, 180,  60), // gold
        ( 60, 220, 100), // green
        (180,  60, 220), // purple
        (220, 120,  60), // orange
    ];

    // ─── Game loop ───────────────────────────────────────────────────────
    'running: loop {
        let frame_start = Instant::now();

        // ── Input ────────────────────────────────────────────────────────
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(kc), .. } => match kc {
                    Keycode::Escape => {
                        show_stats(&mut canvas, &mut event_pump, &stats);
                        break 'running;
                    }
                    Keycode::Up    => try_spawn(&mut vehicles, &mut next_id, Direction::North,
                                                tick, &mut last_spawn[0], palette),
                    Keycode::Down  => try_spawn(&mut vehicles, &mut next_id, Direction::South,
                                                tick, &mut last_spawn[1], palette),
                    Keycode::Left  => try_spawn(&mut vehicles, &mut next_id, Direction::West,
                                                tick, &mut last_spawn[2], palette),
                    Keycode::Right => try_spawn(&mut vehicles, &mut next_id, Direction::East,
                                                tick, &mut last_spawn[3], palette),
                    Keycode::R => {
                        random_mode = !random_mode;
                        println!("Random spawn mode: {}", random_mode);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        // ── Random spawn ─────────────────────────────────────────────────
        if random_mode {
            random_spawn_timer += 1;
            if random_spawn_timer >= RANDOM_SPAWN_INTERVAL {
                random_spawn_timer = 0;
                let dirs = [Direction::North, Direction::South, Direction::West, Direction::East];
                let d = rand::thread_rng().gen_range(0..4);
                try_spawn(&mut vehicles, &mut next_id, dirs[d],
                          tick, &mut last_spawn[d], palette);
            }
        }

        // ── Physics & smart management ───────────────────────────────────
        update(&mut intersection, &mut vehicles, &mut stats, tick);

        // ── Collect finished vehicles ─────────────────────────────────────
        collect_finished(&mut vehicles, &mut stats);

        // ── Render ───────────────────────────────────────────────────────
        draw_world(&mut canvas, &vehicles, random_mode);
        canvas.present();

        tick += 1;

        // ── Frame cap ────────────────────────────────────────────────────
        let elapsed = frame_start.elapsed();
        let target  = Duration::from_millis(FRAME_MS);
        if elapsed < target {
            std::thread::sleep(target - elapsed);
        }
    }
}

// ─── Spawn helper ─────────────────────────────────────────────────────────────

fn try_spawn(
    vehicles: &mut Vec<Vehicle>,
    next_id: &mut u32,
    dir: Direction,
    tick: u64,
    last: &mut u64,
    palette: &[(u8, u8, u8)],
) {
    if tick.saturating_sub(*last) < SPAWN_COOLDOWN_TICKS {
        return;
    }

    let routes = [Route::Left, Route::Straight, Route::Right];
    let route  = routes[rand::thread_rng().gen_range(0..3)];

    let (sx, sy) = Intersection::spawn_position(dir);

    // Reject if another vehicle is already near the entry point
    let too_close = vehicles.iter().any(|v| {
        v.active && {
            let dx = v.x - sx;
            let dy = v.y - sy;
            (dx * dx + dy * dy).sqrt() < SAFE_DISTANCE
        }
    });
    if too_close { return; }

    let waypoints = Intersection::build_waypoints(dir, route);
    let color     = palette[(*next_id as usize) % palette.len()];

    vehicles.push(Vehicle::new(*next_id, sx, sy, dir, route, waypoints, color));
    *next_id += 1;
    *last = tick;
}

// ─── Per-tick simulation update ───────────────────────────────────────────────

fn update(
    intersection: &mut Intersection,
    vehicles: &mut Vec<Vehicle>,
    stats: &mut Stats,
    _tick: u64,
) {
    let n = vehicles.len();

    // ── Look-ahead: slow down if too close to vehicle ahead ──────────────
    for i in 0..n {
        if !vehicles[i].active { continue; }
        let mut min_dist = f32::INFINITY;
        for j in 0..n {
            if i == j || !vehicles[j].active { continue; }
            let dist = vehicles[i].distance_to(&vehicles[j]);
            if dist < 120.0 && dist < min_dist {
                let dxi = vehicles[j].x - vehicles[i].x;
                let dyi = vehicles[j].y - vehicles[i].y;
                let fwd_x =  vehicles[i].angle.to_radians().sin();
                let fwd_y = -vehicles[i].angle.to_radians().cos();
                let dot = dxi * fwd_x + dyi * fwd_y;
                if dot > 0.0 {
                    min_dist = dist;
                }
            }
        }

        if min_dist < SAFE_DISTANCE {
            let new_speed = vehicles[i].speed.downshift();
            vehicles[i].speed = new_speed;
        } else if min_dist > SAFE_DISTANCE * 1.8 {
            if vehicles[i].speed == Speed::Slow {
                vehicles[i].speed = Speed::Normal;
            }
        }
    }

    // ── Reservation management ────────────────────────────────────────────
    for i in 0..n {
        if !vehicles[i].active { continue; }

        let in_zone = Intersection::in_conflict_zone(&vehicles[i]);

        if in_zone {
            vehicles[i].in_intersection = true;
            if vehicles[i].reservation_id.is_none() {
                let granted = intersection.try_reserve(vehicles, i);
                if !granted {
                    let spd = vehicles[i].speed.downshift();
                    vehicles[i].speed = spd;
                }
            }
        } else if vehicles[i].in_intersection {
            Intersection::release_reservation(&mut vehicles[i]);
        } else {
            // Approaching: request reservation if within 150px of zone centre
            let close_to_zone = {
                let v = &vehicles[i];
                (v.x - 400.0).abs() < 150.0 && (v.y - 400.0).abs() < 150.0
            };
            if close_to_zone && vehicles[i].reservation_id.is_none() {
                let granted = intersection.try_reserve(vehicles, i);
                if !granted {
                    let spd = vehicles[i].speed.downshift();
                    vehicles[i].speed = spd;
                } else if vehicles[i].speed == Speed::Slow {
                    vehicles[i].speed = Speed::Normal;
                }
            }
        }
    }

    // ── Close-call detection ──────────────────────────────────────────────
    for i in 0..n {
        if !vehicles[i].active { continue; }
        for j in (i + 1)..n {
            if !vehicles[j].active { continue; }
            let d = vehicles[i].distance_to(&vehicles[j]);
            if d < CLOSE_CALL_DIST {
                stats.record_close_call();
            }
        }
    }

    // ── Advance vehicles ──────────────────────────────────────────────────
    for v in vehicles.iter_mut() {
        if v.active {
            v.advance();
        }
    }
}

// ─── Remove finished vehicles and record stats ────────────────────────────────

fn collect_finished(vehicles: &mut Vec<Vehicle>, stats: &mut Stats) {
    vehicles.retain(|v| {
        if !v.active {
            if let Some(ct) = v.crossing_time() {
                stats.record_vehicle(v.speed.pixels_per_tick(), ct);
            }
            false
        } else {
            true
        }
    });
}

// ─── Statistics screen ────────────────────────────────────────────────────────

fn show_stats(
    canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    event_pump: &mut sdl2::EventPump,
    stats: &Stats,
) {
    println!("\n--- Simulation ended ---");
    for line in stats.report() {
        println!("{}", line);
    }
    println!("--- Press any key to quit ---");

    draw_stats_screen(canvas, stats);
    canvas.present();

    'stats: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'stats,
                Event::KeyDown { .. } => break 'stats,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(16));
    }
}
