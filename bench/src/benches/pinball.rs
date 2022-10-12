macro_rules! bench {
    ($channel_name:ident) => {
        pub mod $channel_name {
            use std::num::NonZeroU32;
            use std::sync::atomic::{AtomicUsize, Ordering};
            use std::sync::Arc;
            use std::time::Instant;

            use oorandom;

            use crate::channel_shims::$channel_name::channel;
            use crate::executor_shims::Executor;
            use crate::{BenchIterator, BenchResult};

            pub fn bench<E: Executor>(samples: NonZeroU32) -> BenchIterator {
                const TOTAL_PATH_LENGTH: usize = 1_000_000;
                const GRAPH_COUNT: usize = 61;
                const NODES_PER_GRAPHS: usize = 13;
                let results =
                    [1, 3, 7, 17, 41, 101, 241]
                        .into_iter()
                        .map(move |visitor_count: usize| {
                            let total_messages =
                                (TOTAL_PATH_LENGTH / visitor_count) * visitor_count * GRAPH_COUNT;

                            let throughput: Vec<_> = (0..samples.get())
                                .map(|_| {
                                    let mut executor = E::default();
                                    let total_visitor_path_length =
                                        TOTAL_PATH_LENGTH / visitor_count;

                                    for graph_id in 0..GRAPH_COUNT {
                                        let mut senders = Vec::new();
                                        let mut receivers = Vec::new();

                                        // Build a sender-receiver pair for each graph
                                        // node.
                                        for _ in 0..NODES_PER_GRAPHS {
                                            let (s, r) = channel(visitor_count);
                                            senders.push(s);
                                            receivers.push(r);
                                        }

                                        // Count how many visitors have completed their
                                        // journey through the graph.
                                        let halted_visitors = Arc::new(AtomicUsize::new(0));

                                        // Create one task per graph node.
                                        for (i, mut r) in receivers.into_iter().enumerate() {
                                            // Clone the senders of all other nodes.
                                            let mut other_senders: Vec<_> = senders
                                                .iter()
                                                .enumerate()
                                                .filter_map(|(j, s)| {
                                                    if i != j {
                                                        Some(s.clone())
                                                    } else {
                                                        None
                                                    }
                                                })
                                                .collect();

                                            // Clone the local sender.
                                            let mut s = senders[i].clone();

                                            let seed = graph_id + GRAPH_COUNT * i;
                                            let mut rng = oorandom::Rand64::new(seed as u128);
                                            let halted_visitors = halted_visitors.clone();

                                            executor.spawn(async move {
                                                // The visitors are initially
                                                // distributed as uniformly as
                                                // possible between the nodes.
                                                let visitors =
                                                    if i < visitor_count % NODES_PER_GRAPHS {
                                                        visitor_count / NODES_PER_GRAPHS + 1
                                                    } else {
                                                        visitor_count / NODES_PER_GRAPHS
                                                    };
                                                for _ in 0..visitors {
                                                    let _ = s.send(0usize).await;
                                                }

                                                // All nodes increment the path length
                                                // of the received visitor and propagate
                                                // it to another node randomly.
                                                loop {
                                                    let mut path_length = match r.recv().await {
                                                        // Stop if the wind-down signal
                                                        // is received or if all senders
                                                        // were dropped.
                                                        Some(usize::MAX) | None => break,
                                                        // Retrieve the path length of
                                                        // the visitor.
                                                        Some(v) => v,
                                                    };

                                                    path_length += 1;

                                                    if path_length < total_visitor_path_length {
                                                        // Send the visitor to
                                                        // another random node.
                                                        let target = rng.rand_range(
                                                            0..other_senders.len() as u64,
                                                        );
                                                        other_senders[target as usize]
                                                            .send(path_length)
                                                            .await;
                                                    } else {
                                                        // The visitor has completed its
                                                        // journey.
                                                        let v = halted_visitors
                                                            .fetch_add(1, Ordering::Relaxed);
                                                        // Broadcast the wind-down
                                                        // signal and exit if all
                                                        // visitors are halted.
                                                        if v + 1 == visitor_count {
                                                            for mut s in other_senders {
                                                                s.send(usize::MAX).await
                                                            }
                                                            break;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                    }

                                    let start_time = Instant::now();
                                    executor.join_all();
                                    let duration = Instant::now() - start_time;

                                    total_messages as f64 / duration.as_secs_f64()
                                })
                                .collect();

                            BenchResult::new(
                                String::from("ball count"),
                                visitor_count.to_string(),
                                throughput,
                            )
                        });

                Box::new(results)
            }
        }
    };
}

crate::macros::add_bench!();
