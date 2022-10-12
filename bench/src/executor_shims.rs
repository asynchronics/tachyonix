use std::future::Future;

pub trait Executor: Default {
    fn spawn<T: Future<Output = ()> + Send + 'static>(&mut self, future: T);
    fn join_all(&mut self);
}

pub struct TokioExecutor {
    join_handles: Vec<::tokio::task::JoinHandle<()>>,
    runtime: ::tokio::runtime::Runtime,
}

impl Executor for TokioExecutor {
    fn spawn<T: Future<Output = ()> + Send + 'static>(&mut self, future: T) {
        self.join_handles.push(self.runtime.spawn(future));
    }
    fn join_all(&mut self) {
        let join_handles = std::mem::take(&mut self.join_handles);
        self.runtime.block_on(async move {
            for fut in join_handles {
                fut.await.unwrap();
            }
        });
    }
}

impl Default for TokioExecutor {
    fn default() -> Self {
        let runtime = ::tokio::runtime::Builder::new_multi_thread()
            .build()
            .unwrap();

        Self {
            join_handles: Vec::new(),
            runtime,
        }
    }
}

#[derive(Default)]
pub struct AsyncStdExecutor {
    join_handles: Vec<::async_std::task::JoinHandle<()>>,
}
impl Executor for AsyncStdExecutor {
    fn spawn<T: Future<Output = ()> + Send + 'static>(&mut self, future: T) {
        self.join_handles.push(::async_std::task::spawn(future));
    }
    fn join_all(&mut self) {
        let join_handles = std::mem::take(&mut self.join_handles);
        ::async_std::task::block_on(async move {
            for fut in join_handles {
                fut.await;
            }
        });
    }
}

#[derive(Default)]
pub struct SmolScaleExecutor {
    join_handles: Vec<::async_task::Task<()>>,
}
impl Executor for SmolScaleExecutor {
    fn spawn<T: Future<Output = ()> + Send + 'static>(&mut self, future: T) {
        self.join_handles.push(::smolscale::spawn(future));
    }
    fn join_all(&mut self) {
        let join_handles = std::mem::take(&mut self.join_handles);
        ::smolscale::block_on(async move {
            for fut in join_handles {
                fut.await;
            }
        });
    }
}

pub struct AsynchronixExecutor {
    executor: ::asynchronix::dev_hooks::Executor,
}
impl Executor for AsynchronixExecutor {
    fn spawn<T: Future<Output = ()> + Send + 'static>(&mut self, future: T) {
        self.executor.spawn_and_forget(future);
    }
    fn join_all(&mut self) {
        self.executor.run();
    }
}

impl Default for AsynchronixExecutor {
    fn default() -> Self {
        Self {
            executor: ::asynchronix::dev_hooks::Executor::new(::num_cpus::get()),
        }
    }
}
