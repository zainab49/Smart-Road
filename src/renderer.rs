// renderer.rs — All SDL2 drawing calls

use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::Canvas;
use sdl2::video::Window;

use crate::intersection::{ISECT_X1, ISECT_X2, ISECT_Y1, ISECT_Y2, ISECT_CX, ISECT_CY, WINDOW_W, WINDOW_H};
use crate::vehicle::Vehicle;
use crate::stats::Stats;

const ROAD_COLOR:  Color = Color::RGB(60,  60,  60);
const GRASS_COLOR: Color = Color::RGB(34, 100,  34);
const LANE_COLOR:  Color = Color::RGB(220, 220,  80);
const ZONE_COLOR:  Color = Color::RGB(80,  80,  80);
const ROAD_HALF: i32 = 40;

pub fn draw_world(canvas: &mut Canvas<Window>, vehicles: &[Vehicle], random_mode: bool) {
    // Background (grass)
    canvas.set_draw_color(GRASS_COLOR);
    canvas.clear();

    draw_roads(canvas);
    draw_vehicles(canvas, vehicles);
    draw_hud(canvas, random_mode);
}

fn draw_roads(canvas: &mut Canvas<Window>) {
    // Vertical road (North–South)
    canvas.set_draw_color(ROAD_COLOR);
    let _ = canvas.fill_rect(Rect::new(
        ISECT_CX as i32 - ROAD_HALF, 0,
        (ROAD_HALF * 2) as u32, WINDOW_H as u32,
    ));

    // Horizontal road (East–West)
    let _ = canvas.fill_rect(Rect::new(
        0, ISECT_CY as i32 - ROAD_HALF,
        WINDOW_W as u32, (ROAD_HALF * 2) as u32,
    ));

    // Intersection box (slightly lighter)
    canvas.set_draw_color(ZONE_COLOR);
    let _ = canvas.fill_rect(Rect::new(
        ISECT_X1 as i32, ISECT_Y1 as i32,
        (ISECT_X2 - ISECT_X1) as u32, (ISECT_Y2 - ISECT_Y1) as u32,
    ));

    // Centre dashed lane lines – vertical
    canvas.set_draw_color(LANE_COLOR);
    let dash_h = 20i32;
    let gap    = 15i32;
    let mut y = 0i32;
    while y < WINDOW_H {
        if y + dash_h < ISECT_Y1 as i32 || y > ISECT_Y2 as i32 {
            let _ = canvas.fill_rect(Rect::new(ISECT_CX as i32 - 2, y, 4, dash_h as u32));
        }
        y += dash_h + gap;
    }

    // Centre dashed lane lines – horizontal
    let dash_w = 20i32;
    let mut x = 0i32;
    while x < WINDOW_W {
        if x + dash_w < ISECT_X1 as i32 || x > ISECT_X2 as i32 {
            let _ = canvas.fill_rect(Rect::new(x, ISECT_CY as i32 - 2, dash_w as u32, 4));
        }
        x += dash_w + gap;
    }

    // Stop lines (white bars at intersection entry)
    canvas.set_draw_color(Color::WHITE);
    // North entry
    let _ = canvas.fill_rect(Rect::new(ISECT_CX as i32 - ROAD_HALF, ISECT_Y1 as i32 - 4, (ROAD_HALF) as u32, 4));
    // South entry
    let _ = canvas.fill_rect(Rect::new(ISECT_CX as i32, ISECT_Y2 as i32, (ROAD_HALF) as u32, 4));
    // West entry
    let _ = canvas.fill_rect(Rect::new(ISECT_X1 as i32 - 4, ISECT_CY as i32, 4, ROAD_HALF as u32));
    // East entry
    let _ = canvas.fill_rect(Rect::new(ISECT_X2 as i32, ISECT_CY as i32 - ROAD_HALF, 4, ROAD_HALF as u32));

    // Cardinal labels
    // (text rendering requires TTF; we use simple corner rectangles as markers instead)
    draw_direction_marker(canvas, ISECT_CX as i32, 10, Color::RGB(255,80,80));   // N
    draw_direction_marker(canvas, ISECT_CX as i32, WINDOW_H - 20, Color::RGB(80,180,255)); // S
    draw_direction_marker(canvas, 10, ISECT_CY as i32, Color::RGB(80,255,80));   // W
    draw_direction_marker(canvas, WINDOW_W - 20, ISECT_CY as i32, Color::RGB(255,200,80)); // E
}

fn draw_direction_marker(canvas: &mut Canvas<Window>, x: i32, y: i32, color: Color) {
    canvas.set_draw_color(color);
    let _ = canvas.fill_rect(Rect::new(x - 8, y - 8, 16, 16));
}

fn draw_vehicles(canvas: &mut Canvas<Window>, vehicles: &[Vehicle]) {
    for v in vehicles {
        if !v.active { continue; }

        let vw = 14u32;
        let vh = 22u32;

        // Draw rotated rectangle using copy_ex simulation:
        // SDL2's Canvas::copy_ex is for textures; for filled rects we draw
        // a rotated quad by computing the four corners manually.
        draw_rotated_rect(canvas, v.x, v.y, vw as f32, vh as f32, v.angle,
            Color::RGB(v.color_r, v.color_g, v.color_b));

        // Draw a small white dot at the vehicle nose (front)
        canvas.set_draw_color(Color::WHITE);
        let nose_x = v.x + v.angle.to_radians().sin() * (vh as f32 / 2.0 - 3.0);
        let nose_y = v.y - v.angle.to_radians().cos() * (vh as f32 / 2.0 - 3.0);
        let _ = canvas.fill_rect(Rect::new(nose_x as i32 - 2, nose_y as i32 - 2, 4, 4));
    }
}

/// Rasterise a rotated rectangle using line-drawing (no texture needed)
fn draw_rotated_rect(canvas: &mut Canvas<Window>, cx: f32, cy: f32, w: f32, h: f32, angle_deg: f32, color: Color) {
    let rad = angle_deg.to_radians();
    let cos = rad.cos();
    let sin = rad.sin();

    let hw = w / 2.0;
    let hh = h / 2.0;

    let corners_local: [(f32, f32); 4] = [
        (-hw, -hh), ( hw, -hh), ( hw,  hh), (-hw,  hh),
    ];

    let corners: Vec<Point> = corners_local.iter().map(|(lx, ly)| {
        let rx = cx + lx * cos - ly * sin;
        let ry = cy + lx * sin + ly * cos;
        Point::new(rx as i32, ry as i32)
    }).collect();

    canvas.set_draw_color(color);
    // Fill by drawing horizontal spans (simple scanline)
    let min_y = corners.iter().map(|p| p.y).min().unwrap_or(0);
    let max_y = corners.iter().map(|p| p.y).max().unwrap_or(0);

    for y in min_y..=max_y {
        let mut xs: Vec<i32> = Vec::new();
        let n = corners.len();
        for i in 0..n {
            let a = corners[i];
            let b = corners[(i + 1) % n];
            if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                let t = (y - a.y) as f32 / (b.y - a.y) as f32;
                xs.push((a.x as f32 + t * (b.x - a.x) as f32) as i32);
            }
        }
        if xs.len() >= 2 {
            xs.sort_unstable();
            let _ = canvas.fill_rect(Rect::new(xs[0], y, (xs[xs.len()-1] - xs[0]).unsigned_abs() + 1, 1));
        }
    }
}

fn draw_hud(canvas: &mut Canvas<Window>, random_mode: bool) {
    // Simple coloured indicator for random-spawn mode
    if random_mode {
        canvas.set_draw_color(Color::RGB(255, 100, 0));
        let _ = canvas.fill_rect(Rect::new(WINDOW_W - 20, 5, 15, 15));
    }
}

// ─── Statistics screen ───────────────────────────────────────────────────────

pub fn draw_stats_screen(canvas: &mut Canvas<Window>, stats: &Stats) {
    canvas.set_draw_color(Color::RGB(15, 15, 30));
    canvas.clear();

    let report = stats.report();
    // Render each line as a coloured bar (TTF not available without extra feature;
    // we draw rectangles whose widths encode numeric values, plus thin white lines
    // to indicate each stat row).

    for (i, line) in report.iter().enumerate() {
        // Print to stdout so the user can read the full report
        println!("{}", line);

        // Visual: draw a bar row
        let y = 80 + i as i32 * 70;
        canvas.set_draw_color(Color::RGB(40, 40, 80));
        let _ = canvas.fill_rect(Rect::new(60, y, 680, 50));

        // Accent bar length proportional to index (decorative)
        let accent = Color::RGB(
            (80 + i * 28) as u8,
            (120 + i * 15) as u8,
            200,
        );
        canvas.set_draw_color(accent);
        let bar_w = ((i + 1) * 90).min(660) as u32;
        let _ = canvas.fill_rect(Rect::new(62, y + 2, bar_w, 46));
    }

    // Title bar
    canvas.set_draw_color(Color::RGB(255, 200, 0));
    let _ = canvas.fill_rect(Rect::new(60, 20, 680, 40));
}
