use std::{cell::RefCell, rc::Rc};

use ratatui::prelude::{Constraint, Layout};

use crate::cli::console::components::{Component, Empty, Header, Menu, StatusBar};

pub struct MainLayout<'a> {
    header: Header,
    menu: Menu<'a>,
    status_bar: StatusBar,

    body: Rc<RefCell<dyn Component>>,
}

impl<'a> MainLayout<'a> {
    pub fn new() -> Self {
        Self {
            header: Header::new(),
            menu: Menu::new(vec!["Main", "Blocks"]),
            status_bar: StatusBar::new(),
            body: Rc::new(RefCell::new(Empty {})),
        }
    }

    pub fn set_body(&mut self, body: Rc<RefCell<dyn Component>>) {
        self.body = body;
    }
}

impl<'a> Component for MainLayout<'a> {
    fn draw(
        &mut self,
        f: &mut crate::cli::console::tui::Frame<'_>,
        area: ratatui::prelude::Rect,
        theme: &crate::cli::console::theme::Theme,
    ) -> color_eyre::eyre::Result<()> {
        let rects = Layout::default()
            .constraints(
                [
                    Constraint::Length(1), // header
                    Constraint::Min(3),    // body
                    Constraint::Length(1), // menu
                    Constraint::Length(1), // status bar
                ]
                .as_ref(),
            )
            .split(area);

        self.header.draw(f, rects[0], theme)?;
        self.body.borrow_mut().draw(f, rects[1], theme)?;
        self.menu.draw(f, rects[2], theme)?;
        self.status_bar.draw(f, rects[3], theme)?;

        Ok(())
    }
}
