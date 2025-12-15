pub mod home;
pub use home::Home;

#[derive(Debug)]
pub enum Screen {
    Loading,
    Home(home::Home),
}
