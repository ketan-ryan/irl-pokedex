SCRAPES POKEMON HOME SPRITES (3D MODELS)

source: https://projectpokemon.org/home/docs/spriteindex_148/home-sprites-gen-{X}-r{Y} WHERE X AND Y ARE 1-8 (GENERATION) AND 128-135 (ID)

run pokemondb_scraper.py
run projectpokemon_scraper.py
run combiner.py
run downloader.py

files are named like so (all are 512x512 except after #898, then 256x256)
poke_capture_0001_000_mf_n_00000000_f_n.png 
0001 = pokedex index
000 = variant (000 is default)
mf = default form; can be uk (unkown), md (male), fd (female), mo (male-variant), fo (female-variant)
first n = default form; can be g (gigantamax)
second n = default form; can be r (shiny)

note pokemon after #898 were obtained from pokemondb not projectpokemon so img types are different and no variants