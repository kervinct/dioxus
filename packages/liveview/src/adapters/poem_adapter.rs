use std::sync::Arc;

use crate::{interpreter_glue, LiveViewError, LiveViewSocket, LiveviewRouter};
use futures_util::{SinkExt, StreamExt};
use poem::{
    get, handler,
    listener::TcpListener,
    web::{
        websocket::{Message, WebSocket, WebSocketStream},
        Data, Html,
    },
    EndpointExt, IntoResponse, Request, Route, Server,
};

/// Convert an Poem WebSocket into a `LiveViewSocket`.
///
/// This is required to launch a LiveView app using the Poem web framework.
pub fn poem_socket(ws: WebSocketStream) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, std::io::Error>) -> Result<Vec<u8>, LiveViewError> {
    Ok(message
        .map_err(|_| LiveViewError::SendingFailed)?
        .as_bytes()
        .to_vec())
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, std::io::Error> {
    Ok(Message::Binary(message))
}

impl LiveviewRouter for Route {
    fn create_default_liveview_router() -> Self {
        Route::new()
    }

    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static,
    ) -> Self {
        #[handler]
        async fn index(req: &Request) -> impl IntoResponse {
            let path = req.uri().path();
            let glue = if path.len() == 1 {
                interpreter_glue(&"/ws")
            } else {
                interpreter_glue(&format!("{path}/ws"))
            };
            let title = crate::app_title();

            Html(format!(
                r#"
        <!DOCTYPE html>
        <html>
            <head> <title>{title}</title>  </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#,
            ))
        }

        #[handler]
        async fn ws(ws: WebSocket, app: Data<&LiveviewApp>) -> impl IntoResponse {
            let pool = app.pool.clone();
            let app = app.app.clone();

            ws.on_upgrade(move |socket| async move {
                _ = pool
                    .launch_virtualdom(poem_socket(socket), move || app())
                    .await;
            })
        }

        #[derive(Clone)]
        struct LiveviewApp {
            app: Arc<dyn Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static>,
            pool: Arc<crate::LiveViewPool>,
        }

        let app = Arc::new(app);
        let view = Arc::new(crate::LiveViewPool::new());

        self.at(route, get(index)).nest(
            format!("{route}/ws"),
            get(ws).data(LiveviewApp { app, pool: view }),
        )
    }

    async fn start(self, address: impl Into<std::net::SocketAddr>) {
        if let Err(err) = Server::new(TcpListener::bind(address.into()))
            .run(self)
            .await
        {
            eprintln!("Failed to start poem server: {}", err);
        }
    }
}
