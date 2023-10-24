use ratatui::style::{Style, Color, Modifier};

mod default;

use default::default as theme_default;

#[derive(Debug)]
pub struct ColorScheme {
    main_bg: Color,
    main_fg: Color,

    header_fg: Color,
    header_bg: Color,

    popup_fg: Color,
    popup_bg: Color,
    popup_title_fg: Color,

    menu_bg: Color,
    menu_fg: Color,
    menu_divider_fg: Color,
    menu_highlight_bg: Color,
    menu_highlight_fg: Color,

    status_bar_fg: Color,

    warning_title_fg: Color,
    warning_border_fg: Color,
    warning_text_1_fg: Color,
    warning_text_2_fg: Color,

    // 'blocks' tab
    blocks_title_fg: Color,
    blocks_border_fg: Color,
    blocks_table_header_fg: Color,
    //blocks_table_row_gauge_fg: Color,
    //blocks_table_row_gauge_bg: Color,
    blocks_table_row_top_1_fg: Color,
    blocks_table_row_top_2_fg: Color,
    blocks_table_row_bottom_fg: Color,
    blocks_table_row_highlight_bg: Color,
}

impl From<&str> for ColorScheme {
    fn from(value: &str) -> Self {
        match value.to_ascii_uppercase().as_str() {
            "DEFAULT" => theme_default(),
            _ => theme_default(),
        }
    }
}

#[derive(Debug)]
pub struct Theme {
    pub main: Style,

    pub header: Style,

    pub popup_fg_bg: Style,
    pub popup_title: Style,
    
    pub menu: Style,
    pub menu_divider: Style,
    pub menu_highlight: Style,

    pub warning_title: Style,
    pub warning_border: Style,
    pub warning_text_1: Style,
    pub warning_text_2: Style,

    // 'blocks' tab
    pub blocks_title: Style,
    pub blocks_border: Style,
    pub blocks_table_header: Style,
    //pub blocks_table_row_gauge: Style,
    pub blocks_table_row_top_1: Style,
    pub blocks_table_row_top_2: Style,
    pub blocks_table_row_bottom: Style,
    pub blocks_table_row_highlight: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Theme::new(theme_default(), true)
    }
}

impl Theme {
    pub fn new(cs: ColorScheme, draw_background: bool) -> Self {
        let main_bg = if draw_background { cs.main_bg } else { Color::Reset };
        let menu_bg = if draw_background { cs.menu_bg } else { Color::Reset };
        let menu_highlight_bg = if draw_background { cs.menu_highlight_bg } else { Color::Reset };

        Theme {
            main: Style::default().fg(cs.main_fg).bg(main_bg),

            header: Style::default().fg(cs.header_fg).bg(cs.header_bg),

            popup_fg_bg: Style::default().fg(cs.popup_fg).bg(cs.popup_bg),
            popup_title: Style::default().fg(cs.popup_title_fg),

            menu: Style::default().fg(cs.menu_fg).bg(menu_bg),
            menu_divider: Style::default().fg(cs.menu_divider_fg),
            menu_highlight: Style::default()
                .fg(cs.menu_highlight_fg)
                .bg(menu_highlight_bg)
                .add_modifier(Modifier::BOLD),

            warning_title: Style::default().fg(cs.warning_title_fg),
            warning_border: Style::default().fg(cs.warning_border_fg),
            warning_text_1: Style::default().fg(cs.warning_text_1_fg),
            warning_text_2: Style::default().fg(cs.warning_text_2_fg),

            blocks_title: Style::default().fg(cs.blocks_title_fg),
            blocks_border: Style::default().fg(cs.blocks_border_fg),
            blocks_table_header: Style::default()
                .fg(cs.blocks_table_header_fg)
                .add_modifier(Modifier::BOLD),
            //blocks_table_row_gauge: Style::default().fg(cs.blocks_table_row_gauge_fg).bg(cs.calls_table_row_gauge_bg),
            blocks_table_row_top_1: Style::default().fg(cs.blocks_table_row_top_1_fg).bg(main_bg),
            blocks_table_row_top_2: Style::default().fg(cs.blocks_table_row_top_2_fg).bg(main_bg),
            blocks_table_row_bottom: Style::default().fg(cs.blocks_table_row_bottom_fg).bg(main_bg),
            blocks_table_row_highlight: Style::default()
                .bg(cs.blocks_table_row_highlight_bg)
                .add_modifier(Modifier::BOLD),
        }
    }

    pub fn color_table_cell(style_start: Style, style_stop: Style, index: u8, size: u16) -> Style {
        let bg = style_start.bg.unwrap_or(Color::Reset);
        let start_color = style_start.fg.unwrap_or(Color::Reset);
        let start_r: f32;
        let start_g: f32;
        let start_b: f32;

        match start_color {
            Color::Rgb(r, g, b) => {
                start_r = r as f32;
                start_g = g as f32;
                start_b = b as f32;
            }
            _ => return Style::default().fg(start_color).bg(bg)
        }

        let min = start_r.min(start_g).min(start_b) as u8;

        let stop_color = style_stop.fg.unwrap_or(Color::Rgb(min, min, min));

        let stop_r: f32;
        let stop_g: f32;
        let stop_b: f32;

        match stop_color {
            Color::Rgb(r, g, b) => {
                stop_r = r as f32;
                stop_g = g as f32;
                stop_b = b as f32;
            }
            _ => return Style::default().fg(start_color).bg(bg)
        }

        let s = match size {
            0..=12 => 12,
            13..=30 => size,
            _ => 30
        } as f32;

        let idx = index as f32;

        let r = (start_r - (((start_r - stop_r).max(0.0) / s) * idx)).max(stop_r);
        let g = (start_g - (((start_g - stop_g).max(0.0) / s) * idx)).max(stop_g);
        let b = (start_b - (((start_b - stop_b).max(0.0) / s) * idx)).max(stop_b);

        Style::default().fg(Color::Rgb(r as u8, g as u8, b as u8)).bg(bg)
    }
}