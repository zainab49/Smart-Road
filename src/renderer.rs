// renderer.rs - Rendering for simulation scene and end stats scene.

use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Canvas, Texture, TextureCreator};
use sdl2::surface::Surface;
use sdl2::video::{Window, WindowContext};

use crate::animation::{AnimDir, FRAME_COLS, FRAME_ROWS};
use crate::intersection::{GUIDE_LINE_SPACING, ISECT_CX, ISECT_CY};
use crate::stats::Stats;
use crate::vehicle::Vehicle;

const CAR_W: u32 = 26;
const CAR_H: u32 = 46;

pub struct SpriteSheet {
    pub texture: Texture,
    frame_rects: Vec<Rect>,
}

impl SpriteSheet {
    fn frame_rect(&self, dir: AnimDir, frame: u8) -> Rect {
        let cols = FRAME_COLS as usize;
        let idx = (dir as usize * cols) + (frame as usize % cols);
        self.frame_rects[idx]
    }
}

pub struct GameTextures {
    pub map: Texture,
    /// v1.png .. v7.png, indexed 0..6.
    pub sprites: Vec<SpriteSheet>,
}

impl GameTextures {
    pub fn load(tc: &TextureCreator<WindowContext>) -> Self {
        let map = load_png_texture(tc, include_bytes!("assets/map2.png"), false);

        Self {
            map,
            sprites: vec![
                load_sprite_sheet(tc, include_bytes!("assets/v1.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v2.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v3.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v4.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v5.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v6.png")),
                load_sprite_sheet(tc, include_bytes!("assets/v7.png")),
            ],
        }
    }
}

struct DecodedPng {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

fn decode_png(bytes: &[u8]) -> DecodedPng {
    let mut decoder = png::Decoder::new(bytes);
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().expect("PNG read_info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("PNG next_frame");

    let rgba: Vec<u8> = match info.color_type {
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

    DecodedPng {
        width: info.width,
        height: info.height,
        rgba,
    }
}

fn texture_from_rgba(
    tc: &TextureCreator<WindowContext>,
    rgba: &mut [u8],
    w: u32,
    h: u32,
    alpha_blend: bool,
) -> Texture {
    let surface = Surface::from_data(rgba, w, h, w * 4, PixelFormatEnum::ABGR8888)
        .expect("Surface::from_data");

    let mut tex = tc
        .create_texture_from_surface(&surface)
        .expect("create_texture_from_surface");

    if alpha_blend {
        tex.set_blend_mode(BlendMode::Blend);
    }

    tex
}

fn load_png_texture(tc: &TextureCreator<WindowContext>, bytes: &[u8], alpha_blend: bool) -> Texture {
    let mut decoded = decode_png(bytes);
    texture_from_rgba(
        tc,
        &mut decoded.rgba,
        decoded.width,
        decoded.height,
        alpha_blend,
    )
}

fn load_sprite_sheet(tc: &TextureCreator<WindowContext>, bytes: &[u8]) -> SpriteSheet {
    let mut decoded = decode_png(bytes);
    let frame_rects = build_sprite_frame_rects(decoded.width, decoded.height, &decoded.rgba);
    let texture = texture_from_rgba(tc, &mut decoded.rgba, decoded.width, decoded.height, true);

    SpriteSheet {
        texture,
        frame_rects,
    }
}

fn build_sprite_frame_rects(sheet_w: u32, sheet_h: u32, rgba: &[u8]) -> Vec<Rect> {
    let col_sums = axis_alpha_sums_x(sheet_w, sheet_h, rgba);
    let row_sums = axis_alpha_sums_y(sheet_w, sheet_h, rgba);

    let col_bounds = axis_bounds_from_alpha(sheet_w as usize, FRAME_COLS as usize, &col_sums);
    let row_bounds = axis_bounds_from_alpha(sheet_h as usize, FRAME_ROWS as usize, &row_sums);

    let mut rects = Vec::with_capacity((FRAME_COLS as usize) * (FRAME_ROWS as usize));
    for row in 0..FRAME_ROWS as usize {
        let y0 = row_bounds[row] as u32;
        let y1 = row_bounds[row + 1] as u32;
        let h = y1.saturating_sub(y0).max(1);

        for col in 0..FRAME_COLS as usize {
            let x0 = col_bounds[col] as u32;
            let x1 = col_bounds[col + 1] as u32;
            let w = x1.saturating_sub(x0).max(1);
            rects.push(Rect::new(x0 as i32, y0 as i32, w, h));
        }
    }

    rects
}

fn axis_alpha_sums_x(w: u32, h: u32, rgba: &[u8]) -> Vec<u32> {
    let mut sums = vec![0u32; w as usize];
    let wu = w as usize;

    for y in 0..h as usize {
        for x in 0..wu {
            let i = ((y * wu + x) * 4) + 3;
            sums[x] += rgba[i] as u32;
        }
    }

    sums
}

fn axis_alpha_sums_y(w: u32, h: u32, rgba: &[u8]) -> Vec<u32> {
    let mut sums = vec![0u32; h as usize];
    let wu = w as usize;

    for y in 0..h as usize {
        let mut sum = 0u32;
        for x in 0..wu {
            let i = ((y * wu + x) * 4) + 3;
            sum += rgba[i] as u32;
        }
        sums[y] = sum;
    }

    sums
}

fn axis_bounds_from_alpha(len: usize, bins: usize, sums: &[u32]) -> Vec<usize> {
    debug_assert_eq!(len, sums.len());

    let mut bounds = Vec::with_capacity(bins + 1);
    bounds.push(0);

    let mut prev_cut = 0usize;
    for k in 1..bins {
        let ideal = k * len / bins;
        let mut radius = len / (bins * 2);
        radius = radius.max(2);

        let mut start = ideal.saturating_sub(radius);
        let mut end = (ideal + radius).min(len.saturating_sub(1));

        let remaining_cuts = bins - k;
        let max_cut = len.saturating_sub(remaining_cuts);

        if start <= prev_cut {
            start = prev_cut + 1;
        }
        if start > max_cut {
            start = max_cut;
        }
        if end < start {
            end = start;
        }
        if end > max_cut {
            end = max_cut;
        }

        let mut best = start;
        let mut best_val = sums[start];
        for i in start..=end {
            if sums[i] < best_val {
                best = i;
                best_val = sums[i];
            }
        }

        bounds.push(best);
        prev_cut = best;
    }

    bounds.push(len);
    bounds
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

    draw_pale_conflict_grid(canvas, win_w, win_h);

    for vehicle in vehicles {
        draw_vehicle(canvas, vehicle, textures);
    }
}

fn draw_pale_conflict_grid(canvas: &mut Canvas<Window>, win_w: u32, win_h: u32) {
    let _ = canvas.set_blend_mode(BlendMode::Blend);

    // Draw exactly 7 vertical and 7 horizontal lines centered on the map.
    canvas.set_draw_color(Color::RGBA(245, 236, 170, 80));
    for i in -3..=3 {
        let x = (ISECT_CX + i as f32 * GUIDE_LINE_SPACING).round() as i32;
        let _ = canvas.draw_line(
            Point::new(x, 0),
            Point::new(x, win_h as i32),
        );
        let y = (ISECT_CY + i as f32 * GUIDE_LINE_SPACING).round() as i32;
        let _ = canvas.draw_line(
            Point::new(0, y),
            Point::new(win_w as i32, y),
        );
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
    let src = sheet.frame_rect(vehicle.anim.dir, vehicle.anim.frame);

    let dest = Rect::new(
        vehicle.x as i32 - CAR_W as i32 / 2,
        vehicle.y as i32 - CAR_H as i32 / 2,
        CAR_W,
        CAR_H,
    );

    // Keep row-based facing and apply a small continuous rotation offset for smooth turning.
    let base = row_base_angle(vehicle.anim.dir);
    let rot = normalize_degrees(vehicle.angle as f64 - base);

    let _ = canvas.copy_ex(&sheet.texture, Some(src), Some(dest), rot, None, false, false);
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
