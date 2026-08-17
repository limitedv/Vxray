use url::Url;

fn main() {
    let link = "vless://00000000-0000-0000-0000-000000000000@1.2.3.4:443?type=xhttp";
    let url = Url::parse(link).unwrap();
    println!("Host: {:?}", url.host_str());
}
