use ratatui::style::Color;

use super::ColorScheme;

#[allow(dead_code)]
enum ColorPalette {
    Black1,
    Black2,
    White,
    Cyan,
    Red,
    Green,
    Yellow,
    Blue,
    Orange,
    Pink,
    Purple,
}

impl From<ColorPalette> for Color {
    fn from(n: ColorPalette) -> Self {
        use ColorPalette::*;
        match n {
            White => Color::Rgb(248, 248, 242),
            Cyan => Color::Rgb(139, 233, 253),
            Red => Color::Rgb(255, 85, 85),
            Green => Color::Rgb(80, 250, 123),
            Yellow => Color::Rgb(241, 250, 140),
            Blue => Color::Rgb(98, 114, 164),
            Orange => Color::Rgb(255, 184, 108),
            Black1 => Color::Rgb(40, 42, 54),
            Black2 => Color::Rgb(68, 71, 90),
            Pink => Color::Rgb(255, 121, 198),
            Purple => Color::Rgb(189, 147, 249),
        }
    }
}

pub fn default() -> ColorScheme {
    use ColorPalette::*;

    ColorScheme {
        main_bg: Black1.into(),
        main_fg: White.into(),

        header_fg: Pink.into(),
        header_bg: Black2.into(),

        popup_fg: White.into(),
        popup_bg: Black2.into(),
        popup_title_fg: White.into(),

        menu_bg: Black1.into(),
        menu_fg: White.into(),
        menu_divider_fg: Black2.into(),
        menu_highlight_bg: Black1.into(),
        menu_highlight_fg: Orange.into(),

        status_bar_fg: Blue.into(),

        warning_title_fg: White.into(),
        warning_border_fg: Black2.into(),
        warning_text_1_fg: Red.into(),
        warning_text_2_fg: Green.into(),

        // 'blocks' tab
        blocks_title_fg: White.into(),
        blocks_border_fg: Black2.into(),
        blocks_table_header_fg: Cyan.into(),
        //blocks_table_row_gauge_fg: Blue,
        //blocks_table_row_gauge_bg: Gray,
        blocks_table_row_top_1_fg: White.into(),
        blocks_table_row_top_2_fg: Blue.into(),
        blocks_table_row_bottom_fg: Black2.into(),
        blocks_table_row_highlight_bg: Orange.into(),
    }
}
