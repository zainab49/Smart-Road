// renderer.rs — Renders the pixel-art map.png as background, vehicles on top.

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::intersection::{WINDOW_W, WINDOW_H};
use crate::vehicle::Vehicle;
use crate::stats::Stats;

// ─── Texture bundle ───────────────────────────────────────────────────────────

pub struct SeaTextures {
    pub map: Texture,
}

impl SeaTextures {
    pub fn load(tc: &TextureCreator<WindowContext>) -> Self {
        SeaTextures {
            map: load_png(tc, include_bytes!("assets/map.png")),
        }
    }
}

/// Decode an embedded PNG → SDL2 Texture using the pure-Rust `png` crate.
fn load_png(tc: &TextureCreator<WindowContext>, bytes: &[u8]) -> Texture {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info().expect("PNG read_info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("PNG next_frame");

    let w = info.width;
    let h = info.height;

    let mut rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb  => buf[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 0xFF])
            .collect(),
        other => panic!("Unsupported PNG colour type: {:?}", other),
    };

    // ABGR8888 on little-endian x86 stores bytes as [R,G,B,A] in memory —
    // exactly what the PNG decoder outputs.
    let surface = Surface::from_data(&mut rgba, w, h, w * 4, PixelFormatEnum::ABGR8888)
        .expect("Surface::from_data");
    tc.create_texture_from_surface(&surface)
        .expect("create_texture_from_surface")
}

// ─── Public entry points ──────────────────────────────────────────────────────

pub fn draw_world(
    canvas:      &mut Canvas<Window>,
    vehicles:    &[Vehicle],
    random_mode: bool,
    textures:    &SeaTextures,
) {
    // Stretch map.png to fill the entire window
    canvas
        .copy(
            &textures.map,
            None,
            Some(Rect::new(0, 0, WINDOW_W as u32, WINDOW_H as u32)),
        )
        .expect("map copy");

    draw_vehicles(canvas, vehicles);
    draw_hud(canvas, random_mode);
}

// ─── Vehicle rendering ────────────────────────────────────────────────────────

fn draw_vehicles(canvas: &mut Canvas<Window>, vehicles: &[Vehicle]) {
    for v in vehicles {
        if !v.active { continue; }

        // Body
        draw_rotated_rect(
            canvas, v.x, v.y, 14.0, 22.0, v.angle,
            Color::RGB(v.color_r, v.color_g, v.color_b),
        );

        // Dark outline for contrast against the sea
        draw_rotated_rect(
            canvas, v.x, v.y, 16.0, 24.0, v.angle,
            Color::RGBA(0, 0, 0, 120),
        );
        draw_rotated_rect(
            canvas, v.x, v.y, 14.0, 22.0, v.angle,
            Color::RGB(v.color_r, v.color_g, v.color_b),
        );

        // White nose dot (direction indicator)
        canvas.set_draw_color(Color::WHITE);
        let nose_x = v.x + v.angle.to_radians().sin() * 8.0;
        let nose_y = v.y - v.angle.to_radians().cos() * 8.0;
        let _ = canvas.fill_rect(Rect::new(nose_x as i32 - 2, nose_y as i32 - 2, 4, 4));
    }
}

/// Rasterise a filled rotated rectangle via scanline — no texture needed.
fn draw_rotated_rect(
    canvas:    &mut Canvas<Window>,
    cx: f32,   cy: f32,
    w:  f32,   h:  f32,
    angle_deg: f32,
    color:     Color,
) {
    let rad = angle_deg.to_radians();
    let (s, c) = (rad.sin(), rad.cos());
    let (hw, hh) = (w / 2.0, h / 2.0);

    let corners: Vec<Point> = [(-hw,-hh),(hw,-hh),(hw,hh),(-hw,hh)]
        .iter()
        .map(|(lx, ly)| Point::new(
            (cx + lx * c - ly * s) as i32,
            (cy + lx * s + ly * c) as i32,
        ))
        .collect();

    canvas.set_draw_color(color);
    let min_y = corners.iter().map(|p| p.y).min().unwrap_or(0);
    let max_y = corners.iter().map(|p| p.y).max().unwrap_or(0);
    let n = corners.len();

    for y in min_y..=max_y {
        let mut xs: Vec<i32> = Vec::new();
        for i in 0..n {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
                let t = (y - a.y) as f32 / (b.y - a.y) as f32;
                xs.push((a.x as f32 + t * (b.x - a.x) as f32) as i32);
            }
        }
        if xs.len() >= 2 {
            xs.sort_unstable();
            let span = (xs[xs.len()-1] - xs[0]).unsigned_abs() + 1;
            let _ = canvas.fill_rect(Rect::new(xs[0], y, span, 1));
        }
    }
}

// ─── HUD ─────────────────────────────────────────────────────────────────────

fn draw_hud(canvas: &mut Canvas<Window>, random_mode: bool) {
    if random_mode {
        canvas.set_draw_color(Color::RGB(255, 100, 0));
        let _ = canvas.fill_rect(Rect::new(WINDOW_W - 20, 5, 15, 15));
    }
}

// ─── Statistics screen ────────────────────────────────────────────────────────

pub fn draw_stats_screen(canvas: &mut Canvas<Window>, stats: &Stats) {
    canvas.set_draw_color(Color::RGB(15, 15, 30));
    canvas.clear();

    let report = stats.report();
    for (i, line) in report.iter().enumerate() {
        println!("{}", line);
        let y = 80 + i as i32 * 70;
        canvas.set_draw_color(Color::RGB(40, 40, 80));
        let _ = canvas.fill_rect(Rect::new(60, y, 680, 50));
        let accent = Color::RGB(
            (80 + i * 28).min(255) as u8,
            (120 + i * 15).min(255) as u8,
            200,
        );
        canvas.set_draw_color(accent);
        let _ = canvas.fill_rect(Rect::new(62, y + 2, ((i + 1) * 90).min(660) as u32, 46));
    }
    canvas.set_draw_color(Color::RGB(255, 200, 0));
    let _ = canvas.fill_rect(Rect::new(60, 20, 680, 40));
}
