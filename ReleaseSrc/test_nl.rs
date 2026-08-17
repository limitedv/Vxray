use url::Url;
fn main() {
    let link = "vless://foo@bar.com:443\n";
    let res = Url::parse(link);
    println!("{:?}", res);
}
