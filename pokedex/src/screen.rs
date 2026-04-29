pub mod browse_pokedex;
pub mod home;
pub mod register;

pub use browse_pokedex::PokedexBrowser;
pub use home::Home;
pub use register::Register;

#[derive(Debug)]
pub enum Screen {
    Loading,
    Home(home::Home),
    Register(register::Register),
    PokedexBrowser(browse_pokedex::PokedexBrowser),
}
