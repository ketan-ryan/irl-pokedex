SCRAPES POKEMON TCG CARDS IMAGES AND AUTO-CROPS


tcg_scraper.py creates a json file with all image urls. (note please change nidoran's name for male and female to nidoran-m or nidoran-f in the json)

the TCG api is extremely fickle and will fail numerous times on valid searches so it will take a couple re-runs to get all pokemon

as a way to help, i have created json files that list how many pokemon only have 1, 2, 3, or 4 cards (onlyfive.json is not complete) so you can skip them once you obtain extremely

in addition, on top of everything when you search nidoran with the gender char it will likely fail so search for "nidoran" and you will obtain all (you just have to sort them back into their respective json entries)

with pokemon that have spaces in the name, use the other query search and uncomment q=query (in safe_card_query) and the query itself

KNOWN DOWNLOAD FAILURES:
Failed to download https://images.pokemontcg.io/mcd15/8.png: 404 Client Error: Not Found for url: https://images.pokemontcg.io/mcd15/8.png
Failed to download https://images.pokemontcg.io/xy12/112.png: [Errno 22] Invalid argument: "G:\\My Drive\\IRL Pokedex\\TCG imgs\\crops\\doduo\\Imakuni?'s Doduo-xy12-112.png"
Failed to download https://images.pokemontcg.io/dp6/82.png: [Errno 22] Invalid argument: 'G:\\My Drive\\IRL Pokedex\\TCG imgs\\crops\\unown\\Unown [?]-dp6-82.png'
Failed to download https://images.pokemontcg.io/ex10/question.png: [Errno 22] Invalid argument: 'G:\\My Drive\\IRL Pokedex\\TCG imgs\\crops\\unown\\Unown-ex10-?.png'

just uncomment the line where we replace ? with _

tcg_scraper.py
downloader.py