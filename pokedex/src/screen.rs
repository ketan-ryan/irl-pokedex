pub mod home;
pub mod register;

pub use home::Home;
pub use register::Register;

#[derive(Debug)]
pub enum Screen {
    Loading,
    Home(home::Home),
    Register(register::Register),
}
