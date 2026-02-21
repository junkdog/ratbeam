use ratatui::prelude::Color;

/// Converts a [`Color`] to a 24-bit RGB value, with a fallback for reset colors.
pub(crate) fn to_rgb(color: Color, reset_fallback_rgb: u32) -> u32 {
    match color {
        Color::Rgb(r, g, b) => ((r as u32) << 16) | ((g as u32) << 8) | b as u32,
        Color::Reset => reset_fallback_rgb,
        Color::Black => 0x000000,
        Color::Red => 0x800000,
        Color::Green => 0x008000,
        Color::Yellow => 0x808000,
        Color::Blue => 0x000080,
        Color::Magenta => 0x800080,
        Color::Cyan => 0x008080,
        Color::Gray => 0xc0c0c0,
        Color::DarkGray => 0x808080,
        Color::LightRed => 0xFF0000,
        Color::LightGreen => 0x00FF00,
        Color::LightYellow => 0xFFFF00,
        Color::LightBlue => 0x0000FF,
        Color::LightMagenta => 0xFF00FF,
        Color::LightCyan => 0x00FFFF,
        Color::White => 0xFFFFFF,
        Color::Indexed(code) => indexed_color_to_rgb(code),
    }
}

/// Converts an indexed color (0-255) to an RGB value.
fn indexed_color_to_rgb(index: u8) -> u32 {
    match index {
        // Basic 16 colors (0-15)
        0..=15 => {
            const BASIC_COLORS: [u32; 16] = [
                0x000000, // 0: black
                0xCD0000, // 1: red
                0x00CD00, // 2: green
                0xCDCD00, // 3: yellow
                0x0000EE, // 4: blue
                0xCD00CD, // 5: magenta
                0x00CDCD, // 6: cyan
                0xE5E5E5, // 7: white
                0x7F7F7F, // 8: bright Black
                0xFF0000, // 9: bright Red
                0x00FF00, // 10: bright Green
                0xFFFF00, // 11: bright Yellow
                0x5C5CFF, // 12: bright Blue
                0xFF00FF, // 13: bright Magenta
                0x00FFFF, // 14: bright Cyan
                0xFFFFFF, // 15: bright White
            ];
            BASIC_COLORS[index as usize]
        }

        // 216-color cube (16-231)
        16..=231 => {
            let cube_index = index - 16;
            let r = cube_index / 36;
            let g = (cube_index % 36) / 6;
            let b = cube_index % 6;

            let to_rgb = |n: u8| -> u32 {
                if n == 0 { 0 } else { 55 + 40 * n as u32 }
            };

            to_rgb(r) << 16 | to_rgb(g) << 8 | to_rgb(b)
        }

        // 24 grayscale colors (232-255)
        232..=255 => {
            let gray_index = index - 232;
            let gray = (8 + gray_index * 10) as u32;
            (gray << 16) | (gray << 8) | gray
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_color_packing() {
        assert_eq!(to_rgb(Color::Rgb(0xFF, 0x00, 0x80), 0), 0xFF0080);
        assert_eq!(to_rgb(Color::Rgb(0, 0, 0), 0), 0x000000);
        assert_eq!(to_rgb(Color::Rgb(255, 255, 255), 0), 0xFFFFFF);
    }

    #[test]
    fn reset_uses_fallback() {
        assert_eq!(to_rgb(Color::Reset, 0xABCDEF), 0xABCDEF);
        assert_eq!(to_rgb(Color::Reset, 0x000000), 0x000000);
    }

    #[test]
    fn named_ansi_colors() {
        assert_eq!(to_rgb(Color::Black, 0), 0x000000);
        assert_eq!(to_rgb(Color::White, 0), 0xFFFFFF);
        assert_eq!(to_rgb(Color::Red, 0), 0x800000);
        assert_eq!(to_rgb(Color::LightRed, 0), 0xFF0000);
        assert_eq!(to_rgb(Color::LightGreen, 0), 0x00FF00);
        assert_eq!(to_rgb(Color::LightBlue, 0), 0x0000FF);
    }

    #[test]
    fn indexed_basic_16() {
        assert_eq!(indexed_color_to_rgb(0), 0x000000);  // black
        assert_eq!(indexed_color_to_rgb(1), 0xCD0000);  // red
        assert_eq!(indexed_color_to_rgb(15), 0xFFFFFF); // bright white
    }

    #[test]
    fn indexed_color_cube() {
        // Index 16 = (0,0,0) -> black
        assert_eq!(indexed_color_to_rgb(16), 0x000000);
        // Index 21 = (0,0,5) -> blue 0x0000ff
        assert_eq!(indexed_color_to_rgb(21), 0x0000FF);
        // Index 196 = (5,0,0) -> red 0xff0000
        assert_eq!(indexed_color_to_rgb(196), 0xFF0000);
        // Index 231 = (5,5,5) -> white 0xffffff
        assert_eq!(indexed_color_to_rgb(231), 0xFFFFFF);
    }

    #[test]
    fn indexed_grayscale() {
        // First grayscale (232) = gray level 8
        assert_eq!(indexed_color_to_rgb(232), 0x080808);
        // Last grayscale (255) = gray level 238
        assert_eq!(indexed_color_to_rgb(255), 0xEEEEEE);
    }

    #[test]
    fn indexed_colors_match_xterm() {
        // Spot-check against xterm 256-color table
        const XTERM_SAMPLES: [(u8, u32); 8] = [
            (68, 0x5f87d7),
            (88, 0x870000),
            (124, 0xaf0000),
            (160, 0xd70000),
            (208, 0xff8700),
            (240, 0x585858),
            (248, 0xa8a8a8),
            (254, 0xe4e4e4),
        ];

        for (idx, expected) in XTERM_SAMPLES {
            assert_eq!(
                indexed_color_to_rgb(idx),
                expected,
                "Mismatch for indexed color {idx}"
            );
        }
    }
}
