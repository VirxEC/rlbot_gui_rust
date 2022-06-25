use tokio::net::TcpStream;

// Code snippet taken from https://github.com/jesusprubio/online
// License: MIT
// Code was taken due to lack up updates & a lack of eyes from the community (potential security flaw)
// Code also changed from using async-std to tokio and simplified

// Captive portals: https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/API/captivePortal
const ADDRS: [&str; 2] = [
    // - http://clients3.google.com/generate_204
    "clients3.google.com:80",
    // - http://detectportal.firefox.com/success.txt
    "detectportal.firefox.com:80",
];

/// Check if the user has an internet connection
pub async fn check() -> bool {
    TcpStream::connect(ADDRS[0]).await.is_ok() || TcpStream::connect(ADDRS[1]).await.is_ok()
}
