pub mod tachyonix {
    use ::tachyonix as tachyonix_crate;

    #[derive(Clone)]
    pub struct Sender<T> {
        inner: tachyonix_crate::Sender<T>,
    }
    impl<T: std::fmt::Debug> Sender<T> {
        pub async fn send(&mut self, message: T) {
            self.inner.send(message).await.unwrap();
        }
    }

    pub struct Receiver<T> {
        inner: tachyonix_crate::Receiver<T>,
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            self.inner.recv().await.ok()
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = tachyonix_crate::channel(capacity);
        (Sender { inner: s }, Receiver { inner: r })
    }
}

pub mod flume {
    use ::flume as flume_crate;

    #[derive(Clone)]
    pub struct Sender<T> {
        inner: flume_crate::Sender<T>,
    }
    impl<T> Sender<T> {
        pub async fn send(&mut self, message: T) {
            self.inner.send_async(message).await.unwrap();
        }
    }

    pub struct Receiver<T> {
        inner: flume_crate::Receiver<T>,
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            self.inner.recv_async().await.ok()
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = flume_crate::bounded(capacity);
        (Sender { inner: s }, Receiver { inner: r })
    }
}

pub mod async_channel {
    use ::async_channel as async_channel_crate;

    #[derive(Clone)]
    pub struct Sender<T> {
        inner: async_channel_crate::Sender<T>,
    }
    impl<T> Sender<T> {
        pub async fn send(&mut self, message: T) {
            self.inner.send(message).await.unwrap();
        }
    }

    pub struct Receiver<T> {
        inner: async_channel_crate::Receiver<T>,
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            self.inner.recv().await.ok()
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = async_channel_crate::bounded(capacity);
        (Sender { inner: s }, Receiver { inner: r })
    }
}

pub mod tokio_mpsc {
    use ::tokio as tokio_crate;

    use std::fmt::Debug;

    #[derive(Clone)]
    pub struct Sender<T> {
        inner: tokio_crate::sync::mpsc::Sender<T>,
    }
    impl<T: Debug> Sender<T> {
        pub async fn send(&mut self, message: T) {
            self.inner.send(message).await.unwrap();
        }
    }

    pub struct Receiver<T> {
        inner: tokio_crate::sync::mpsc::Receiver<T>,
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            self.inner.recv().await
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = tokio_crate::sync::mpsc::channel(capacity);
        (Sender { inner: s }, Receiver { inner: r })
    }
}

pub mod postage_mpsc {
    use ::postage as postage_crate;
    use ::postage::sink::Sink;
    use ::postage::stream::Stream;

    use std::fmt::Debug;

    #[derive(Clone)]
    pub struct Sender<T> {
        inner: postage_crate::mpsc::Sender<T>,
    }
    impl<T: Debug> Sender<T> {
        pub async fn send(&mut self, message: T) {
            self.inner.send(message).await.unwrap();
        }
    }

    pub struct Receiver<T> {
        inner: postage_crate::mpsc::Receiver<T>,
    }
    impl<T> Receiver<T> {
        pub async fn recv(&mut self) -> Option<T> {
            self.inner.recv().await
        }
    }

    pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
        let (s, r) = postage_crate::mpsc::channel(capacity);
        (Sender { inner: s }, Receiver { inner: r })
    }
}
