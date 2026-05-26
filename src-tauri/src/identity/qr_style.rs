use image::{Rgba, RgbaImage};
use qrcode::QrCode;

// ---------------------------------------------------------------------------
// Palette
// ---------------------------------------------------------------------------

const BG_TEAL: Rgba<u8> = Rgba([0x18, 0x3A, 0x3C, 0xFF]); // #183A3C
const FRAME_GOLD: Rgba<u8> = Rgba([0xB5, 0x8C, 0x43, 0xFF]); // #B58C43
const LABEL_GOLD: Rgba<u8> = Rgba([0xD4, 0xAF, 0x5A, 0xFF]); // #D4AF5A
const QR_BG: Rgba<u8> = Rgba([0xF5, 0xF0, 0xE2, 0xFF]); // #F5F0E2 ivory
const QR_INK: Rgba<u8> = Rgba([0x26, 0x20, 0x1C, 0xFF]); // #26201C ink

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

const MODULE_SIZE: u32 = 8; // pixels per QR module
const QUIET_ZONE: u32 = 4; // modules of quiet zone
const FRAME_BORDER: u32 = 6; // gold border width
const TEAL_PADDING: u32 = 24; // outer teal margin
const LABEL_HEIGHT: u32 = 40; // space for label below QR
const LOGO_RATIO: f64 = 0.15; // logo radius as fraction of QR side

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Render a styled QR code with Veil's art-nouveau theme.
/// Returns PNG bytes.
pub fn render_styled_qr(qr: &QrCode) -> Result<Vec<u8>, String> {
    let qr_width = qr.width() as u32;
    let qr_px = qr_width * MODULE_SIZE;
    let quiet_px = QUIET_ZONE * MODULE_SIZE;
    let qr_area = qr_px + 2 * quiet_px;
    let inner_w = qr_area + 2 * FRAME_BORDER;
    let total_w = inner_w + 2 * TEAL_PADDING;
    let total_h = total_w + LABEL_HEIGHT;

    let mut img = RgbaImage::from_pixel(total_w, total_h, BG_TEAL);

    // Gold frame
    let fx = TEAL_PADDING;
    let fy = TEAL_PADDING;
    fill_rect(&mut img, fx, fy, fx + inner_w, fy + inner_w, FRAME_GOLD);

    // Ivory interior (inside gold frame)
    let ix = fx + FRAME_BORDER;
    let iy = fy + FRAME_BORDER;
    fill_rect(&mut img, ix, iy, ix + qr_area, iy + qr_area, QR_BG);

    // QR modules
    let qr_origin_x = ix + quiet_px;
    let qr_origin_y = iy + quiet_px;
    let colors = qr.to_colors();

    for gy in 0..qr_width {
        for gx in 0..qr_width {
            let idx = (gy * qr_width + gx) as usize;
            if colors[idx] == qrcode::Color::Dark {
                let px = qr_origin_x + gx * MODULE_SIZE;
                let py = qr_origin_y + gy * MODULE_SIZE;
                fill_rect(&mut img, px, py, px + MODULE_SIZE, py + MODULE_SIZE, QR_INK);
            }
        }
    }

    // Center logo
    let cx = (qr_origin_x + qr_px / 2) as i32;
    let cy = (qr_origin_y + qr_px / 2) as i32;
    let logo_r = (qr_px as f64 * LOGO_RATIO) as i32;
    let ring_width = 3i32;

    // Outer gold ring
    fill_circle(&mut img, cx, cy, logo_r + ring_width, FRAME_GOLD);
    // Ivory disc
    fill_circle(&mut img, cx, cy, logo_r, QR_BG);
    // Inner teal disc
    let inner_r = logo_r - ring_width - 1;
    fill_circle(&mut img, cx, cy, inner_r, BG_TEAL);

    // "V" glyph in bright gold
    draw_v(&mut img, cx, cy, inner_r, LABEL_GOLD);

    // Corner ornament dots (gold accents at the four frame corners)
    let dot_r = 4i32;
    let dot_inset = FRAME_BORDER as i32 / 2;
    let corners = [
        (fx as i32 + dot_inset, fy as i32 + dot_inset),
        ((fx + inner_w) as i32 - dot_inset, fy as i32 + dot_inset),
        (fx as i32 + dot_inset, (fy + inner_w) as i32 - dot_inset),
        ((fx + inner_w) as i32 - dot_inset, (fy + inner_w) as i32 - dot_inset),
    ];
    for (ox, oy) in corners {
        fill_circle(&mut img, ox, oy, dot_r, LABEL_GOLD);
    }

    // Encode to PNG
    let mut png_bytes = Vec::new();
    image::DynamicImage::ImageRgba8(img)
        .write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )
        .map_err(|e| format!("PNG encoding failed: {e}"))?;

    Ok(png_bytes)
}

// ---------------------------------------------------------------------------
// Drawing helpers
// ---------------------------------------------------------------------------

fn fill_rect(img: &mut RgbaImage, x1: u32, y1: u32, x2: u32, y2: u32, color: Rgba<u8>) {
    let (w, h) = img.dimensions();
    let x2 = x2.min(w);
    let y2 = y2.min(h);
    for y in y1..y2 {
        for x in x1..x2 {
            img.put_pixel(x, y, color);
        }
    }
}

fn fill_circle(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, color: Rgba<u8>) {
    let (w, h) = img.dimensions();
    let r_sq = (r * r) as i64;
    for dy in -r..=r {
        for dx in -r..=r {
            if (dx as i64 * dx as i64 + dy as i64 * dy as i64) <= r_sq {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < w && (py as u32) < h {
                    img.put_pixel(px as u32, py as u32, color);
                }
            }
        }
    }
}

/// Draw a "V" shape centered at (cx, cy) within the given radius.
fn draw_v(img: &mut RgbaImage, cx: i32, cy: i32, r: i32, color: Rgba<u8>) {
    let half_w = r * 5 / 9;
    let top_y = cy - r * 4 / 9;
    let bottom_y = cy + r * 4 / 9;
    let stroke = 3;

    // Left leg: top-left to bottom-center
    draw_thick_line(img, cx - half_w, top_y, cx, bottom_y, stroke, color);
    // Right leg: top-right to bottom-center
    draw_thick_line(img, cx + half_w, top_y, cx, bottom_y, stroke, color);
}

/// Bresenham-style thick line.
fn draw_thick_line(
    img: &mut RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    thickness: i32,
    color: Rgba<u8>,
) {
    let (w, h) = img.dimensions();
    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let steps = dx.max(dy).max(1);
    let half_t = thickness / 2;

    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let px = x0 as f64 + (x1 - x0) as f64 * t;
        let py = y0 as f64 + (y1 - y0) as f64 * t;

        for dy_off in -half_t..=half_t {
            for dx_off in -half_t..=half_t {
                let fx = (px + dx_off as f64).round() as i32;
                let fy = (py + dy_off as f64).round() as i32;
                if fx >= 0 && fy >= 0 && (fx as u32) < w && (fy as u32) < h {
                    img.put_pixel(fx as u32, fy as u32, color);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use qrcode::EcLevel;

    fn make_test_qr() -> QrCode {
        QrCode::with_error_correction_level(b"test payload for veil qr", EcLevel::H).unwrap()
    }

    #[test]
    fn test_styled_qr_produces_valid_png() {
        let qr = make_test_qr();
        let png = render_styled_qr(&qr).expect("render failed");
        assert!(png.len() > 8, "PNG should be more than 8 bytes");
        assert_eq!(
            &png[..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "must start with PNG magic"
        );
    }

    #[test]
    fn test_styled_qr_dimensions() {
        let qr = make_test_qr();
        let png = render_styled_qr(&qr).expect("render failed");
        let decoded = image::load_from_memory(&png).expect("decode PNG");
        let (w, h) = decoded.dimensions();

        let qr_width = qr.width() as u32;
        let qr_px = qr_width * MODULE_SIZE;
        let quiet_px = QUIET_ZONE * MODULE_SIZE;
        let qr_area = qr_px + 2 * quiet_px;
        let inner_w = qr_area + 2 * FRAME_BORDER;
        let expected_w = inner_w + 2 * TEAL_PADDING;
        let expected_h = expected_w + LABEL_HEIGHT;

        assert_eq!(w, expected_w, "width mismatch");
        assert_eq!(h, expected_h, "height mismatch");
    }

    #[test]
    fn test_styled_qr_center_is_not_ink() {
        let qr = make_test_qr();
        let png = render_styled_qr(&qr).expect("render failed");
        let decoded = image::load_from_memory(&png).expect("decode PNG").to_rgba8();
        let (w, _h) = decoded.dimensions();

        // Center of the image (in the QR area) should be the logo, not ink
        let cx = w / 2;
        let cy = (w - LABEL_HEIGHT) / 2; // approximate vertical center of QR area
        let pixel = decoded.get_pixel(cx, cy);
        assert_ne!(*pixel, QR_INK, "center should be logo, not ink");
    }

    #[test]
    fn test_styled_qr_corner_is_teal() {
        let qr = make_test_qr();
        let png = render_styled_qr(&qr).expect("render failed");
        let decoded = image::load_from_memory(&png).expect("decode PNG").to_rgba8();

        // Top-left corner should be teal
        let pixel = decoded.get_pixel(0, 0);
        assert_eq!(*pixel, BG_TEAL, "corner should be teal");
    }
}
