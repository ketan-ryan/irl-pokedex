pub mod browse_pokedex;
pub mod home;
pub mod register;

#[derive(Debug)]
pub enum Screen {
    Loading,
    Home(home::home::Home),
    Register(register::register::Register),
    PokedexBrowser(browse_pokedex::browse_pokedex::PokedexBrowser),
}
