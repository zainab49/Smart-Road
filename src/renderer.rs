// renderer.rs - Rendering for simulation scene and end stats scene.

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::Rect;
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::animation::AnimDir;
use crate::stats::Stats;
use crate::vehicle::Vehicle;

const CAR_W: u32 = 56;
const CAR_H: u32 = 98;

pub struct GameTextures {
    pub map: Texture,
    /// v1.png .. v7.png, indexed 0..6.
    pub sprites: Vec<Texture>,
}

impl GameTextures {
    pub fn load(tc: &TextureCreator<WindowContext>) -> Self {
        Self {
            map: load_png(tc, include_bytes!("assets/map.png"), false),
            sprites: vec![
                load_png(tc, include_bytes!("assets/v1.png"), true),
                load_png(tc, include_bytes!("assets/v2.png"), true),
                load_png(tc, include_bytes!("assets/v3.png"), true),
                load_png(tc, include_bytes!("assets/v4.png"), true),
                load_png(tc, include_bytes!("assets/v5.png"), true),
                load_png(tc, include_bytes!("assets/v6.png"), true),
                load_png(tc, include_bytes!("assets/v7.png"), true),
            ],
        }
    }
}

fn load_png(tc: &TextureCreator<WindowContext>, bytes: &[u8], alpha_blend: bool) -> Texture {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("PNG read_info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("PNG next_frame");

    let w = info.width;
    let h = info.height;

    let mut rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()]
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 0xFF])
            .collect(),
        png::ColorType::Grayscale => buf[..info.buffer_size()]
            .iter()
            .flat_map(|g| [*g, *g, *g, 0xFF])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf[..info.buffer_size()]
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        other => panic!("Unsupported PNG color type: {:?}", other),
    };

    let surface = Surface::from_data(&mut rgba, w, h, w * 4, PixelFormatEnum::ABGR8888)
        .expect("Surface::from_data");

    let mut tex = tc
        .create_texture_from_surface(&surface)
        .expect("create_texture_from_surface");

    if alpha_blend {
        tex.set_blend_mode(BlendMode::Blend);
    }

    tex
}

pub fn draw_simulation(
    canvas: &mut Canvas<Window>,
    vehicles: &[Vehicle],
    textures: &GameTextures,
    win_w: u32,
    win_h: u32,
) {
    canvas
        .copy(&textures.map, None, Some(Rect::new(0, 0, win_w, win_h)))
        .expect("map copy");

    for vehicle in vehicles {
        draw_vehicle(canvas, vehicle, textures);
    }
}

pub fn draw_stats_screen(canvas: &mut Canvas<Window>, stats: &Stats, win_w: u32, win_h: u32) {
    canvas.set_draw_color(Color::RGB(16, 18, 24));
    canvas.clear();

    let panel = Rect::new(70, 70, win_w.saturating_sub(140), win_h.saturating_sub(140));
    canvas.set_draw_color(Color::RGB(34, 40, 56));
    let _ = canvas.fill_rect(panel);

    canvas.set_draw_color(Color::RGB(85, 96, 125));
    let _ = canvas.draw_rect(panel);

    let lines = stats.dashboard_lines();

    let mut y = panel.y() + 30;
    for (idx, line) in lines.iter().enumerate() {
        let (scale, color) = if idx == 0 {
            (4, Color::RGB(218, 226, 255))
        } else {
            (3, Color::RGB(209, 216, 240))
        };

        draw_text_3x5(canvas, panel.x() + 30, y, line, scale, color);
        y += if idx == 0 { 42 } else { 34 };
    }

    draw_text_3x5(
        canvas,
        panel.x() + 30,
        panel.bottom() - 30,
        "PRESS ESC TO CLOSE",
        2,
        Color::RGB(170, 180, 208),
    );
}

fn draw_vehicle(canvas: &mut Canvas<Window>, vehicle: &Vehicle, textures: &GameTextures) {
    if textures.sprites.is_empty() {
        return;
    }

    let sheet = &textures.sprites[(vehicle.sprite_index as usize).min(textures.sprites.len() - 1)];
    let sheet_info = sheet.query();
    let src = vehicle.anim.src_rect(sheet_info.width, sheet_info.height);

    let dest = Rect::new(
        vehicle.x as i32 - CAR_W as i32 / 2,
        vehicle.y as i32 - CAR_H as i32 / 2,
        CAR_W,
        CAR_H,
    );

    // Keep row-based facing and apply a small continuous rotation offset for smooth turning.
    let base = row_base_angle(vehicle.anim.dir);
    let rot = normalize_degrees(vehicle.angle as f64 - base);

    let _ = canvas.copy_ex(sheet, Some(src), Some(dest), rot, None, false, false);
}

fn row_base_angle(dir: AnimDir) -> f64 {
    match dir {
        AnimDir::Up => 0.0,
        AnimDir::Right => 90.0,
        AnimDir::Down => 180.0,
        AnimDir::Left => 270.0,
    }
}

fn normalize_degrees(angle: f64) -> f64 {
    let mut a = angle % 360.0;
    if a > 180.0 {
        a -= 360.0;
    }
    if a < -180.0 {
        a += 360.0;
    }
    a
}

fn draw_text_3x5(
    canvas: &mut Canvas<Window>,
    x: i32,
    y: i32,
    text: &str,
    scale: u32,
    color: Color,
) {
    canvas.set_draw_color(color);

    let mut cursor_x = x;
    for ch in text.chars() {
        let glyph = glyph_3x5(ch);
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                let mask = 1 << (2 - col);
                if bits & mask != 0 {
                    let px = cursor_x + (col as i32 * scale as i32);
                    let py = y + (row as i32 * scale as i32);
                    let _ = canvas.fill_rect(Rect::new(px, py, scale, scale));
                }
            }
        }
        cursor_x += (4 * scale) as i32;
    }
}

fn glyph_3x5(ch: char) -> [u8; 5] {
    match ch.to_ascii_uppercase() {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        ' ' => [0b000, 0b000, 0b000, 0b000, 0b000],
        _ => [0b111, 0b101, 0b010, 0b000, 0b010],
    }
}
