use std::future::IntoFuture;

#[derive(Debug, bevy::ecs::system::Resource)]
pub struct Runtime(tokio::runtime::Handle);

impl Runtime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let handle = runtime.handle().clone();
        std::thread::spawn(move || runtime.block_on(std::future::pending::<()>()));
        Self(handle)
    }

    pub fn spawn_background(&self, fut: impl IntoFuture<IntoFuture: Send + 'static, Output = ()>) {
        let _ = self.0.spawn(fut.into_future());
    }
}
