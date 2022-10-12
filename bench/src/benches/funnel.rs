macro_rules! bench {
    ($channel_name:ident) => {
        pub mod $channel_name {
            use std::num::NonZeroU32;
            use std::time::Instant;

            use crate::channel_shims::$channel_name::channel;
            use crate::executor_shims::Executor;
            use crate::{BenchIterator, BenchResult};

            pub fn bench<E: Executor>(samples: NonZeroU32) -> BenchIterator {
                const MESSAGES_PER_CHANNEL: usize = 1_000_000;
                const CHANNELS: usize = 61;
                const SENDERS_PER_CHANNEL: usize = 13;
                let total_messages =
                    (MESSAGES_PER_CHANNEL / SENDERS_PER_CHANNEL) * SENDERS_PER_CHANNEL * CHANNELS;

                let results = [1, 10, 100, 1000, 10000]
                    .into_iter()
                    .map(move |capacity: usize| {
                        let throughput: Vec<_> = (0..samples.get())
                            .map(|_| {
                                let mut executor = E::default();

                                for _ in 0..CHANNELS {
                                    let (s, mut r) = channel(capacity);

                                    for _ in 0..SENDERS_PER_CHANNEL {
                                        let mut s = s.clone();

                                        let _ = executor.spawn(async move {
                                            for i in 0..MESSAGES_PER_CHANNEL / SENDERS_PER_CHANNEL {
                                                s.send(i).await;
                                            }
                                        });
                                    }

                                    executor.spawn(async move {
                                        for _ in 0..(MESSAGES_PER_CHANNEL / SENDERS_PER_CHANNEL)
                                            * SENDERS_PER_CHANNEL
                                        {
                                            r.recv().await.unwrap();
                                        }
                                    })
                                }

                                let start_time = Instant::now();
                                executor.join_all();
                                let duration = Instant::now() - start_time;

                                total_messages as f64 / duration.as_secs_f64()
                            })
                            .collect();

                        BenchResult::new(String::from("capacity"), capacity.to_string(), throughput)
                    });

                Box::new(results)
            }
        }
    };
}

crate::macros::add_bench!();
