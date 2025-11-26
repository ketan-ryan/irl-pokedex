mod io;

fn main() {
    let pokedex = io::load_dex_entries("../pokedex.json");
    let hydreigon = &pokedex.unwrap()["hydreigon"];
    println!("{:?}", hydreigon.dex_entries);
}
