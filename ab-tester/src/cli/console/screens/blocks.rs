use color_eyre::eyre::Result;
use ratatui::{
    prelude::{Alignment, Constraint, Rect},
    text::{Line, Span},
    widgets::{BorderType, Borders, Row, Table, TableState},
};

use crate::{
    cli::console::{
        components::{title_span, Component},
        data::Block,
        theme::Theme,
        tui::Frame,
    },
    context::BlockCursor,
};

pub struct BlocksScreen<'a> {
    blocks: Vec<Block>,
    _block_cursor: Option<&'a mut BlockCursor>,
    _state: TableState,
}

impl<'a> BlocksScreen<'a> {
    pub fn new() -> Self {
        Self {
            blocks: vec![
                Block {
                    height: 1,
                    hash: "e197ece1f22194d887bb79ea3de0ad125de2736546485dbcc0734714881181e2"
                        .to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 2,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 3,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 4,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 5,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 6,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 7,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 8,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 9,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
                Block {
                    height: 10,
                    hash: "abc123".to_string(),
                    timestamp: "2023-01-01 11:42:12Z".to_string(),
                    tx_count: 1024,
                },
            ],
            _block_cursor: None,
            _state: TableState::default(),
        }
    }
}

impl<'a> Component for BlocksScreen<'a> {
    fn draw(&mut self, f: &mut Frame<'_>, area: Rect, theme: &Theme) -> Result<()> {
        let header_cells = ["height", "hash", "timestamp", "tx count"]
            .iter()
            .map(|h| Line::from(Span::styled(h.to_owned(), theme.blocks_table_header)));

        let header = Row::new(header_cells).height(1).bottom_margin(0);

        let rows = self
            .blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| {
                let style1 = Theme::color_table_cell(
                    theme.blocks_table_row_top_1,
                    theme.blocks_table_row_bottom,
                    idx as u8,
                    area.height.wrapping_sub(1),
                );
                let style2 = Theme::color_table_cell(
                    theme.blocks_table_row_top_2,
                    theme.blocks_table_row_bottom,
                    idx as u8,
                    area.height.wrapping_sub(1),
                );

                vec![
                    Line::from(Span::styled(format!("{}", block.height), style1))
                        .alignment(Alignment::Center),
                    Line::from(Span::styled(block.hash.to_string(), style2)),
                    Line::from(Span::styled(block.timestamp.to_string(), style2)),
                    Line::from(Span::styled(format!("{}", block.tx_count), style1))
                        .alignment(Alignment::Center),
                ]
            })
            .map(Row::new)
            .collect::<Vec<Row>>();

        let table = Table::new(rows)
            .header(header)
            .block(
                ratatui::widgets::Block::default()
                    .borders(Borders::ALL)
                    .border_style(theme.blocks_border)
                    .border_type(BorderType::Rounded)
                    .title(title_span(
                        "Blocks",
                        theme.blocks_title,
                        theme.blocks_border,
                    )),
            )
            .highlight_style(theme.blocks_table_row_highlight)
            .widths(&[
                Constraint::Max(8),
                Constraint::Max(67),
                Constraint::Max(23),
                Constraint::Min(8),
            ]);

        f.render_widget(table, area);
        Ok(())
    }
}
