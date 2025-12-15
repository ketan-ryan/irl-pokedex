use iced::{Task, Element};
use iced::widget::{
    column, text,
};
use iced::{
    Center, Fill
};

#[derive(Debug)]
pub struct Home {
    title: String,
    processing: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    HomeToggled,
    Refresh,
}

pub enum Action {
    None,
    GoHome,
    Run(Task<Message>),
}

impl Home {
    pub fn new() -> (Self, Task<Message>) {
        println!("New home created");
        (
            Self { 
                title: String::from("Home page"),
                processing: false,
            },
            Task::none()
        )
    }

    pub fn update(&mut self, msg: Message) -> Action {
        match msg {
            Message::HomeToggled => {
                self.processing = true;
                Action::None
            }
            Message::Refresh => {
                Action::GoHome
            }
        }
    }

    pub fn top_view(&self) -> Element<'_, Message> {        
        println!("Should be in top screen home now");
        let new_window_button =
            text(format!("top window home screen"));

        column![new_window_button]
            .spacing(50)
            .width(Fill)
            .align_x(Center)
            .width(200)
            .into()
    }

    pub fn bottom_view(&self) -> Element<'_, Message> {
        let window_text = text("Bottom screen text from home");
        column![window_text]
            .spacing(50)
            .width(Fill)
            .align_x(Center)
            .width(200)
            .into()
    }
}