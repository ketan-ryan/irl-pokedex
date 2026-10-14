pub mod browse_pokedex;
pub mod filter;
pub mod home;
pub mod info;
pub mod register;

#[derive(Debug)]
pub enum Screen {
    Loading,
    Home(home::home::Home),
    Register(register::register::Register),
    PokedexBrowser(browse_pokedex::browse_pokedex::PokedexBrowser),
    Filter(filter::filter::Filter),
    Slider(filter::slider::RangeSliderScreen),
    Info(info::info::Info),
}
