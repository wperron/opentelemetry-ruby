use futures_util::future::BoxFuture;
use magnus::{function, Module, Object, RModule, Value};
use opentelemetry::sdk::{self, util::tokio_interval_stream};
use tokio::task::JoinHandle;

use crate::tokio_rb::{self, WrappedStruct};

#[magnus::wrap(class = "OpenTelemetry::SDK::Trace::Runtime")]
#[derive(Debug, Clone)]
pub(crate) struct TraceRuntime {
    pub(crate) inner: tokio_rb::Handle,
}

impl Drop for TraceRuntime {
    fn drop(&mut self) {
        println!("TRACE RUNTIME DROPPED");
    }
}

impl TraceRuntime {
    pub(crate) fn new(rt: Value) -> Result<Self, magnus::Error> {
        let wrapped: WrappedStruct<tokio_rb::Runtime> = rt.try_convert()?;
        let rt = wrapped.get()?;

        Ok(Self { inner: rt.handle() })
    }
}

impl sdk::trace::TraceRuntime for TraceRuntime {
    type Receiver = tokio_stream::wrappers::ReceiverStream<sdk::trace::BatchMessage>;
    type Sender = tokio::sync::mpsc::Sender<sdk::trace::BatchMessage>;

    fn batch_message_channel(&self, capacity: usize) -> (Self::Sender, Self::Receiver) {
        let (sender, receiver) = tokio::sync::mpsc::channel(capacity);
        (
            sender,
            tokio_stream::wrappers::ReceiverStream::new(receiver),
        )
    }
}

impl sdk::runtime::Runtime for TraceRuntime {
    type Interval = tokio_stream::wrappers::IntervalStream;
    type Delay = std::pin::Pin<Box<JoinHandle<()>>>;

    fn interval(&self, duration: std::time::Duration) -> Self::Interval {
        tokio_interval_stream(duration)
    }

    fn spawn(&self, future: BoxFuture<'static, ()>) {
        let _ = self.inner.spawn(future);
    }

    fn delay(&self, duration: std::time::Duration) -> Self::Delay {
        Box::pin(self.inner.spawn(tokio::time::sleep(duration)))
    }
}

// impl From<tokio::runtime::Handle> for TraceRuntime {
//     fn from(h: tokio::runtime::Handle) -> Self {
//         Self { inner: h }
//     }
// }

pub(crate) fn init(module: RModule) -> Result<(), magnus::Error> {
    let class = module.define_class("Runtime", Default::default())?;
    class.define_singleton_method("new", function!(TraceRuntime::new, 1))?;
    Ok(())
}
