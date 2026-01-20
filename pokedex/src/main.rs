mod io;
mod screen;
mod elements;
mod pipeline;

use screen::Screen;
use screen::home;
use elements::grid;

use iced::widget::{
    button, column, space
};
use iced::window::{self};
use iced::{
    Center, Element, Fill, Subscription, Task, Theme
};

use std::collections::BTreeMap;

fn main() -> iced::Result {
    let pokedex: std::collections::HashMap<String, io::PokemonInfo> = io::load_dex_entries("pokedex.json");
    let hydreigon = &pokedex["hydreigon"];
    println!("{:?}", hydreigon.dex_entries);

    iced::daemon(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

struct App {
    windows: BTreeMap<window::Id, WindowType>,
    screen: Screen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowType {
    TopScreen,
    BottomScreen
}

#[derive(Debug, Clone)]
enum Message {
    WindowOpened(window::Id),
    Home(home::Message),
    OpenHome,
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
                screen: Screen::Loading,
                windows: BTreeMap::from([
                    (top_id, WindowType::TopScreen),
                    (bottom_id, WindowType::BottomScreen)
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
                self.open_home()
            }
            Message::Home(message) => {
                let Screen::Home(home) = &mut self.screen else {
                    return Task::none();
                };

                match home.update(message) {
                    home::Action::None => Task::none(),
                    home::Action::GoHome => Task::none(),
                    home::Action::Run(task) => task.map(Message::Home),
                    home::Action::RedrawWindows => Task::none(),
                }
            }
            Message::OpenHome => {
                self.open_home()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        match &self.screen {
            Screen::Home(home) =>
                home.subscription().map(Message::Home),

            _ => Subscription::none(),
        }
    }

    fn open_home(&mut self) -> Task<Message> {
        let (home, task) = screen::Home::new();
        self.screen = Screen::Home(home);
        println!("Set screen to {:?}", self.screen);
        task.map(Message::Home)
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if let Some(window) = self.windows.get(&window_id) {
            match window {
                WindowType::TopScreen => {
                    self.top_view()
                }
                WindowType::BottomScreen => {
                    self.bottom_view()
                }
            }
        } else {
            space().into()
        }
    }

    fn top_view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Home(home) => home.top_view().map(Message::Home),
            Screen::Loading =>  {
                let new_window_button =
                    button("Go home").on_press(Message::OpenHome);

                let content = column![new_window_button]
                    .spacing(50)
                    .width(Fill)
                    .align_x(Center)
                    .width(200);

                content.into()
            },
        }
    }

    fn bottom_view(&self) -> Element<'_, Message> {
        match &self.screen {
            Screen::Home(home) => home.bottom_view().map(Message::Home),
            Screen::Loading =>  {
                space().into()
            },
        }
    }

}