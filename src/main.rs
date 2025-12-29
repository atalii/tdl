use std::env;
use tdl::Access;

#[tokio::main]
async fn main() {
    let client_id = env::var("TDL_CLIENT_ID");
    let client_secret = env::var("TDL_CLIENT_SECRET");
    let streaming_tok = env::var("TDL_BEARER_STREAMING");

    let (Ok(client_id), Ok(client_secret), Ok(streaming_tok)) =
        (client_id, client_secret, streaming_tok)
    else {
        panic!("env vars");
    };

    let access = Access::log_in(&client_id, &client_secret, &streaming_tok)
        .await
        .expect("todo error stuff");

    access.download_track("441696040").await.expect("lol");
}
