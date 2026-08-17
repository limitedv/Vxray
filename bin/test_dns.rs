use std::time::Duration;

fn main() {
    let proxy = reqwest::blocking::Proxy::all("socks5://127.0.0.1:2080").unwrap();
    let client = reqwest::blocking::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    println!("Testing 1.1.1.1...");
    match client.get("https://1.1.1.1").send() {
        Ok(res) => println!("1.1.1.1: {}", res.status()),
        Err(e) => println!("1.1.1.1 error: {}", e),
    }
}
