mod io;

use iced::widget::{
    center, center_x, column, container, scrollable, space, text,
};
use iced::window::{self};
use iced::{
    Center, Element, Fill, Task,
};

use std::collections::BTreeMap;

fn main() -> iced::Result {
    let pokedex: std::collections::HashMap<String, io::PokemonInfo> = io::load_dex_entries("../pokedex.json");
    let hydreigon = &pokedex["hydreigon"];
    println!("{:?}", hydreigon.dex_entries);

    iced::daemon(App::new, App::update, App::view)
        .run()
}

struct App {
    windows: BTreeMap<window::Id, Window>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowType {
    TopScreen,
    BottomScreen
}

#[derive(Debug)]
struct Window {
    title: String,
    window: WindowType,
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let (top_id, open) = window::open(window::Settings {
            size: (640, 480).into(),
            position: window::Position::Specific(iced::Point::new(1000.0, 200.0)),
            resizable: false,
            decorations: false,
            ..window::Settings::default()
        });

        let (bottom_id, open_second) = window::open(window::Settings {
            size: (640, 480).into(),
            position: window::Position::Specific(iced::Point::new(1000.0, 800.0)),
            resizable: false,
            decorations: false,
            ..window::Settings::default()
        });

        (
            Self {
                windows: BTreeMap::from([
                    (top_id, Window::new(WindowType::TopScreen)),
                    (bottom_id, Window::new(WindowType::BottomScreen))
                ]),
            },
            Task::batch([
                open.map(Message::WindowOpened),
                open_second.map(Message::WindowOpened),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(_) => {
                return Task::none();
            }
        }
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if let Some(window) = self.windows.get(&window_id) {
            center(window.view()).into()
        } else {
            space().into()
        }
    }

}

impl Window {
    fn new(window_type: WindowType) -> Self {
        Self {
            title: format!("{:?}", window_type),
            window: window_type,
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let new_window_button =
            text(format!("{:?}", self.window));

        let content = column![new_window_button]
            .spacing(50)
            .width(Fill)
            .align_x(Center)
            .width(200);

        container(scrollable(center_x(content))).padding(10).into()
    }
}