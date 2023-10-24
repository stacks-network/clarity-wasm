use ratatui::{widgets::{TableState, Widget, Cell, Row, Table, Borders, StatefulWidget, BorderType, Paragraph}, text::{Span, Line}, prelude::{Constraint, Alignment}, style::Stylize};

use crate::{cli::console::{theme::Theme, data::Block, widgets::title_span, app::App, Screen}, context::BlockCursor};

pub struct BlocksScreen<'theme, 'data> {
    theme: &'theme Theme,
    blocks: Vec<Block>,
    block_cursor: Option<&'data mut BlockCursor>,
    state: TableState
}

impl<'theme, 'data> BlocksScreen<'theme, 'data> {
    pub fn new(theme: &'theme Theme) -> Self {
        Self {
            theme,
            blocks: vec![
                Block { height: 1, hash: "e197ece1f22194d887bb79ea3de0ad125de2736546485dbcc0734714881181e2".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 2, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 3, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 4, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 5, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 6, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 7, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 8, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 9, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
                Block { height: 10, hash: "abc123".to_string(), timestamp: "2023-01-01 11:42:12Z".to_string(), tx_count: 1024},
            ],
            block_cursor: None,
            state: TableState::default()
        }
    }
}

impl<'theme, 'data> Screen for &mut BlocksScreen<'theme, 'data> {
    fn handle_key_event(&mut self, event: crossterm::event::KeyEvent) {
        todo!()
    }
}

impl<'theme, 'data> Widget for &mut BlocksScreen<'theme, 'data> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer) {
        let header_cells = ["height", "hash", "timestamp", "tx count"]
            .iter()
            .map(|h|
                Line::from(Span::styled(h.to_owned(), self.theme.blocks_table_header))
            );

        let header = Row::new(header_cells)
            .height(1)
            .bottom_margin(0);

        let rows = self.blocks.iter().enumerate().map(|(idx, block)| {
            let style1 = Theme::color_table_cell(self.theme.blocks_table_row_top_1, self.theme.blocks_table_row_bottom, idx as u8, area.height.wrapping_sub(1));
            let style2 = Theme::color_table_cell(self.theme.blocks_table_row_top_2, self.theme.blocks_table_row_bottom, idx as u8, area.height.wrapping_sub(1));

            vec![
                Line::from(Span::styled(format!("{}", block.height), style1)).alignment(Alignment::Center),
                Line::from(Span::styled(block.hash.to_string(), style2)),
                Line::from(Span::styled(block.timestamp.to_string(), style2)),
                Line::from(Span::styled(format!("{}", block.tx_count), style1)).alignment(Alignment::Center),
            ]
        }).map(Row::new).collect::<Vec<Row>>();

        let table = Table::new(rows)
            .header(header)
            .block(ratatui::widgets::Block::default()
                .borders(Borders::ALL)
                .border_style(self.theme.blocks_border)
                .border_type(BorderType::Rounded)
                .title(title_span("Blocks", self.theme.blocks_title, self.theme.blocks_border))
            )
            .highlight_style(self.theme.blocks_table_row_highlight)
            .widths(&[
                Constraint::Max(8),
                Constraint::Max(67),
                Constraint::Max(23),
                Constraint::Min(8)
            ]);

            <Table as StatefulWidget>::render(table, area, buf, &mut self.state)
    }
}