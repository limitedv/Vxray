use url::Url;
use std::collections::HashMap;

fn main() {
    let link = "vless://0e61c812-49f0-4dd3-992d-654d47c52a0f@mio.game2lizard.com:2053?encryption=none&security=tls&sni=liz2.pishi.lat&fp=chrome&insecure=0&allowInsecure=0&type=ws&host=liz2.pishi.lat&path=%2Flizard#%7C%F0%9F%87%A9%F0%9F%87%AA%7C-Germany%20CX";
    let url = Url::parse(link).unwrap();
    let params: HashMap<_, _> = url.query_pairs().into_owned().collect();
    println!("{:#?}", params);
}
