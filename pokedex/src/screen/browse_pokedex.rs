use std::time::Duration;

use iced::{
    Color, Element, Length, Subscription, Task, time,
    widget::{canvas, container},
};

use crate::{elements::grid::Grid, io::PokemonInfo};

// enum State {

// }

#[derive(Debug)]
pub struct PokedexBrowser {
    // state: State,
    grid: Grid,
    window: Vec<PokemonInfo>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick(Duration),
}

pub enum Action {
    None,
    GoHome,
    Run(Task<Message>),
}

impl PokedexBrowser {
    pub fn new() -> (Self, Task<Message>) {
        (
            Self {
                grid: Grid::new(),
                window: Vec::new(),
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::Tick(duration) => {
                self.grid.tick();
                Action::None
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // tick screen for updates ~60fps
        time::every(Duration::from_millis(16))
            .map(|arg0: std::time::Instant| Message::Tick(arg0.elapsed()))
    }

    pub fn top_view(&self) -> Element<'_, Message> {
        container(
            canvas::Canvas::new(&self.grid)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(
                1.0,
                162.0 / 255.0,
                0.0,
            ))),
            text_color: Some(Color::BLACK),
            border: Default::default(),
            shadow: Default::default(),
            snap: Default::default(),
        })
        .into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        container(
            canvas::Canvas::new(&self.grid)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .style(|_| iced::widget::container::Style {
            background: Some(iced::Background::Color(Color::from_rgb(
                1.0,
                162.0 / 255.0,
                0.0,
            ))),
            text_color: Some(Color::BLACK),
            border: Default::default(),
            shadow: Default::default(),
            snap: Default::default(),
        })
        .into()
    }
}
