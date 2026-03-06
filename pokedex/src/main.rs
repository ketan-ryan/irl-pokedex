mod io;
mod screen;
mod elements;
mod ml;

use include_assets::{NamedArchive, include_dir};
use screen::Screen;
use screen::home;
use elements::grid;

use iced::widget::{
    button, column, space
};
use iced::window::{self};
use iced::{
    Center, Element, Fill, Subscription, Task
};

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use crate::io::get_local_path;

fn main() -> iced::Result {
    match get_local_path() {
        Ok(path) => {
            println!("Found local path to be {:?}", path)
        },
        Err(err) => {
            eprintln!("Error getting local path: {:?}", err)
        },
    }

    iced::daemon(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
pub enum PokedexError {
    ConfigNotFound,
    MalformedConfig(String),
    PokedexNotFound(String),
    MalformedPokedex(String),
    AssetsNotFound(String)
}

impl std::fmt::Display for PokedexError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PokedexError::ConfigNotFound => write!(f, "Could not find config next to executable. Please place it in the same directory with name \"pokedex_settings.yaml\""),
            PokedexError::MalformedConfig(e) => write!(f, "Configuration is invalid: {}", e),
            PokedexError::PokedexNotFound(dir) => write!(f, "Could not find pokedex JSON at {}", dir),
            PokedexError::MalformedPokedex(e) => write!(f, "Could not parse Pokedex JSON: {}", e),
            PokedexError::AssetsNotFound(dir) => write!(f, "Could not find assets dir at {}", dir)
        }
    }
}

struct App {
    windows: Option<BTreeMap<window::Id, WindowType>>,
    screen: Screen,
    pokedex: Option<Arc<HashMap<String, io::PokemonInfo>>>,
    error: Option<PokedexError>,
    assets_path: Option<String>
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowType {
    TopScreen,
    BottomScreen
}

#[derive(Debug, Clone)]
enum Message {
    Init,
    WindowOpened(window::Id),
    Home(home::Message),
    OpenHome,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                screen: Screen::Loading,
                windows: None,
                pokedex: None,
                error: None,
                assets_path: None
            },
            Task::done(Message::Init)
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Init => {
                self.load_files()
            }
            Message::WindowOpened(_) => {
                if self.error.is_none() {
                    return self.open_home();
                }
                Task::none()
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

    fn load_files(&mut self) -> Task<Message> {
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

        let window_open_task = Task::batch([
            open.map(Message::WindowOpened),
            open_second.map(Message::WindowOpened),
        ]);

        self.windows = Some(BTreeMap::from([
            (top_id, WindowType::TopScreen),
            (bottom_id, WindowType::BottomScreen)
        ]));

        let config = io::load_settings().map_err(|e| self.error = Some(e));
        if config.is_err() {
            return window_open_task;
        }

        let binding = config.unwrap();
        let filename = binding.get("pokedex_location");
        if filename.is_none() {
            let mcerr = "Could not find key pokedex_location in config. Pokedex cannot be loaded.";
            self.error = Some(PokedexError::MalformedConfig(mcerr.to_string()));
            return window_open_task;
        }

        let entries =  io::load_dex_entries(filename.unwrap()).map_err(|e| self.error = Some(e));
        if entries.is_err() {
            return window_open_task;
        }
        self.pokedex = Some(Arc::new(entries.unwrap()));

        let path = binding.get("sprites_location");
        if path.is_none() {
            let mcerr = "Could not find key sprites_location in config. Assets cannot be loaded.";
            self.error = Some(PokedexError::MalformedConfig(mcerr.to_string()));
            return window_open_task;
        }
        self.assets_path = Some(path.unwrap().to_string());

        window_open_task
    }

    fn open_home(&mut self) -> Task<Message> {
        // If we get here, self.pokedex should be Some
        let (home, task) = screen::Home::new(Arc::clone(self.pokedex.as_ref().unwrap()));
        self.screen = Screen::Home(home);
        task.map(Message::Home)
    }

    fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if let Some(window) = self.windows.clone().expect("Windows should exist!").get(&window_id) {
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
        if self.error.is_none() {
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
        } else {
                iced::widget::container(
            iced::widget::column![
                    iced::widget::text("Fatal Error Detected!")
                        .size(48)
                        .color(iced::Color::from_rgb(1.0, 0.0, 0.0))
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::default()
                        }),
                    iced::widget::text("Program cannot proceed.")
                        .size(48)
                        .color(iced::Color::from_rgb(1.0, 0.0, 0.0))
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..iced::Font::default()
                        }),
                    iced::widget::text(self.error.as_ref().unwrap().to_string())
                        .size(24)
                        .color(iced::Color::from_rgb(0.0, 0.0, 0.0)),
                ]
                .align_x(iced::Center)
                .spacing(16),
            )
            .width(iced::Fill)
            .height(iced::Fill)
            .align_x(iced::Center)
            .align_y(iced::Center)
            .style(|_| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::WHITE)),
                ..Default::default()
            })
        .into()
        }
    }

    fn bottom_view(&self) -> Element<'_, Message> {
        if self.error.is_none() {
            match &self.screen {
                Screen::Home(home) => home.bottom_view().map(Message::Home),
                Screen::Loading =>  {
                    space().into()
                },
            }
        } else {
            let archive = NamedArchive::load(include_dir!("assets"));
            let bytes = archive.get("fainted.jpg").unwrap();
            let handle = iced::widget::image::Handle::from_bytes(bytes.to_vec());
            iced::widget::container(iced::widget::image(handle).width(iced::Fill))
                .width(iced::Fill)
                .height(iced::Fill)
                .style(|_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(iced::Color::BLACK)),
                    ..Default::default()
                })
            .into()
        }
    }
}