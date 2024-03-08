use dioxus::prelude::*;
use dioxus_liveview::LiveviewRouter;
use poem::{listener::TcpListener, Route, Server};

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        div {
            "hello poem! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let app = Route::new().with_app("/test", app);

    println!("Listening on http://{addr}");

    Server::new(TcpListener::bind(&addr))
        .run(app)
        .await
        .unwrap();
}
