use std::mem::swap;
use std::rc::Rc;

use beamterm_core::{CellData, TerminalGrid};
use ratatui::{
    backend::{Backend, ClearType, WindowSize},
    buffer::Cell,
    layout::{Position, Size},
    style::Modifier,
};

use crate::color::to_rgb;
use crate::error::Error;

/// A ratatui [`Backend`] that renders via beamterm-core's GPU-accelerated terminal grid.
///
/// The backend does not own the window or GL lifecycle. The application provides
/// an `Rc<glow::Context>` and a [`TerminalGrid`].
pub struct BeamtermBackend {
    grid: TerminalGrid,
    gl: Rc<glow::Context>,
    cursor_position: Option<Position>,
}

impl BeamtermBackend {
    /// Creates a new [`BeamtermBackend`].
    pub fn new(grid: TerminalGrid, gl: Rc<glow::Context>) -> Self {
        Self {
            grid,
            gl,
            cursor_position: None,
        }
    }

    /// Returns a reference to the terminal grid.
    pub fn grid(&self) -> &TerminalGrid {
        &self.grid
    }

    /// Returns a mutable reference to the terminal grid.
    pub fn grid_mut(&mut self) -> &mut TerminalGrid {
        &mut self.grid
    }
}

impl Backend for BeamtermBackend {
    type Error = Error;

    fn draw<'a, I>(&mut self, content: I) -> Result<(), Self::Error>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let cells = content.map(|(x, y, cell)| (x, y, cell_data(cell)));
        self.grid.update_cells_by_position(cells)?;
        Ok(())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        self.grid.flush_cells(&self.gl)?;
        Ok(())
    }

    fn hide_cursor(&mut self) -> Result<(), Self::Error> {
        self.cursor_position = None;
        Ok(())
    }

    fn show_cursor(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn get_cursor_position(&mut self) -> Result<Position, Self::Error> {
        match self.cursor_position {
            Some(position) => Ok(position),
            None => Ok((0, 0).into()),
        }
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> Result<(), Self::Error> {
        self.cursor_position = Some(position.into());
        Ok(())
    }

    fn clear(&mut self) -> Result<(), Self::Error> {
        let cell_count = self.grid.cell_count();
        let cells = [CellData::new_with_style_bits(" ", 0, 0xffffff, 0x000000)]
            .into_iter()
            .cycle()
            .take(cell_count);

        self.grid.update_cells(&self.gl, cells)?;
        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> Result<(), Self::Error> {
        match clear_type {
            ClearType::All => self.clear(),
            _ => Err(Error::Other("unsupported clear region type".to_string())),
        }
    }

    fn size(&self) -> Result<Size, Self::Error> {
        let (w, h) = self.grid.terminal_size();
        Ok(Size::new(w, h))
    }

    fn window_size(&mut self) -> Result<WindowSize, Self::Error> {
        let (cols, rows) = self.grid.terminal_size();
        let (w, h) = self.grid.canvas_size();

        Ok(WindowSize {
            columns_rows: Size::new(cols, rows),
            pixels: Size::new(w as _, h as _),
        })
    }
}

/// Resolves foreground and background colors for a [`Cell`].
fn resolve_fg_bg_colors(cell: &Cell) -> (u32, u32) {
    let mut fg = to_rgb(cell.fg, 0xffffff);
    let mut bg = to_rgb(cell.bg, 0x000000);

    if cell.modifier.contains(Modifier::REVERSED) {
        swap(&mut fg, &mut bg);
    }

    (fg, bg)
}

/// Converts a ratatui [`Cell`] into a beamterm [`CellData`].
fn cell_data(cell: &Cell) -> CellData<'_> {
    let (fg, bg) = resolve_fg_bg_colors(cell);
    CellData::new_with_style_bits(cell.symbol(), into_glyph_bits(cell.modifier), fg, bg)
}

/// Extracts glyph styling bits from cell modifiers.
///
/// # Bit Layout Reference
///
/// ```plain
/// Modifier bits:     0000_0000_0000_0001  (BOLD at bit 0)
///                    0000_0000_0000_0100  (ITALIC at bit 2)
///                    0000_0000_0000_1000  (UNDERLINED at bit 3)
///                    0000_0001_0000_0000  (CROSSED_OUT at bit 8)
///
/// FontStyle bits:    0000_0100_0000_0000  (Bold as bit 10)
///                    0000_1000_0000_0000  (Italic as bit 11)
/// GlyphEffect bits:  0010_0000_0000_0000  (Underline at bit 13)
///                    0100_0000_0000_0000  (Strikethrough at bit 14)
/// ```
const fn into_glyph_bits(modifier: Modifier) -> u16 {
    let m = modifier.bits();

    (m << 10) & (1 << 10)   // bold
    | (m << 9) & (1 << 11)  // italic
    | (m << 10) & (1 << 13) // underline
    | (m << 6) & (1 << 14)  // strikethrough
}

#[cfg(test)]
mod tests {
    use super::*;
    use beamterm_data::{FontStyle, GlyphEffect};
    use ratatui::style::{Color, Modifier, Style};

    #[test]
    fn font_style_bold() {
        assert_eq!(into_glyph_bits(Modifier::BOLD), FontStyle::Bold as u16);
    }

    #[test]
    fn font_style_italic() {
        assert_eq!(into_glyph_bits(Modifier::ITALIC), FontStyle::Italic as u16);
    }

    #[test]
    fn font_style_bold_italic() {
        assert_eq!(
            into_glyph_bits(Modifier::BOLD | Modifier::ITALIC),
            FontStyle::BoldItalic as u16
        );
    }

    #[test]
    fn glyph_effect_underline() {
        assert_eq!(
            into_glyph_bits(Modifier::UNDERLINED),
            GlyphEffect::Underline as u16
        );
    }

    #[test]
    fn glyph_effect_strikethrough() {
        assert_eq!(
            into_glyph_bits(Modifier::CROSSED_OUT),
            GlyphEffect::Strikethrough as u16
        );
    }

    #[test]
    fn no_modifiers_yields_zero() {
        assert_eq!(into_glyph_bits(Modifier::empty()), 0);
    }

    #[test]
    fn combined_style_and_effect() {
        let bits = into_glyph_bits(Modifier::BOLD | Modifier::UNDERLINED);
        assert_eq!(
            bits,
            FontStyle::Bold as u16 | GlyphEffect::Underline as u16
        );
    }

    #[test]
    fn resolve_colors_default() {
        let cell = Cell::default();
        let (fg, bg) = resolve_fg_bg_colors(&cell);
        // Reset fg -> 0xffffff, Reset bg -> 0x000000
        assert_eq!(fg, 0xffffff);
        assert_eq!(bg, 0x000000);
    }

    #[test]
    fn resolve_colors_explicit_rgb() {
        let mut cell = Cell::default();
        cell.set_style(Style::default().fg(Color::Rgb(255, 0, 128)).bg(Color::Rgb(0, 64, 0)));
        let (fg, bg) = resolve_fg_bg_colors(&cell);
        assert_eq!(fg, 0xff0080);
        assert_eq!(bg, 0x004000);
    }

    #[test]
    fn resolve_colors_reversed_swaps() {
        let mut cell = Cell::default();
        cell.set_style(
            Style::default()
                .fg(Color::Rgb(0xAA, 0xBB, 0xCC))
                .bg(Color::Rgb(0x11, 0x22, 0x33))
                .add_modifier(Modifier::REVERSED),
        );
        let (fg, bg) = resolve_fg_bg_colors(&cell);
        // Colors should be swapped
        assert_eq!(fg, 0x112233);
        assert_eq!(bg, 0xAABBCC);
    }

    #[test]
    fn cell_data_preserves_symbol() {
        let mut cell = Cell::default();
        cell.set_symbol("A");
        cell.set_style(Style::default().fg(Color::White).bg(Color::Black));
        let data = cell_data(&cell);
        // CellData is opaque, but if it constructs without panicking,
        // the style_bits assertion inside new_with_style_bits passed.
        let _ = data;
    }

    #[test]
    fn cell_data_bold_italic_no_panic() {
        let mut cell = Cell::default();
        cell.set_symbol("X");
        cell.set_style(
            Style::default()
                .fg(Color::LightCyan)
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD | Modifier::ITALIC),
        );
        // Must not trigger the debug_assert in CellData::new_with_style_bits
        let _ = cell_data(&cell);
    }
}
