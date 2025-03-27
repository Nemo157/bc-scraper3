use super::client::Client;
use super::Request;
use crossbeam::channel::Receiver;

#[culpa::try_fn]
pub fn run(
    client: Client,
    requests: Receiver<Request>,
) -> eyre::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("web-client".to_owned())
        .spawn(move || {
            for request in &requests {
                match request {
                    Request::Get { url, response } => {
                        let _ = response.send(client.get(&url));
                    }
                    Request::Post {
                        url,
                        data,
                        response,
                    } => {
                        let _ = response.send(client.post(&url, &data));
                    }
                }
            }
        })?
}
