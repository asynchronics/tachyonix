use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::Write;
use std::num::NonZeroU32;

use lexopt::prelude::*;

mod benches;
mod channel_shims;
mod executor_shims;
mod macros;

const HELP_MESSAGE: &str = "\
bench
Benchmark runner for Tachyonix

USAGE:
    bench [OPTIONS] <BENCHNAME>

ARGS:
    <BENCHNAME>    If specified, only run benches containing this string in their names

OPTIONS:
    -h, --help             Print help information
    -l, --list             List available benches
    -s, --samples SAMPLES  Repeat benches SAMPLES times and average the result
    -o, --output FILE      Save the results to FILE
    -e, --exec EXECUTOR    Run the bench with the EXECUTOR runtime;
                           possible values: tokio [default], async-std,
                           smolscale, asynchronix";

macro_rules! add_test {
    ($group:ident, $channel:ident) => {
        (
            stringify!($group),
            stringify!($channel),
            benches::$group::$channel::bench::<crate::executor_shims::TokioExecutor>,
            benches::$group::$channel::bench::<crate::executor_shims::AsyncStdExecutor>,
            benches::$group::$channel::bench::<crate::executor_shims::SmolScaleExecutor>,
            benches::$group::$channel::bench::<crate::executor_shims::AsynchronixExecutor>,
        )
    };
}

#[allow(clippy::type_complexity)]
const BENCHES: &[(
    &str,
    &str,
    fn(NonZeroU32) -> BenchIterator,
    fn(NonZeroU32) -> BenchIterator,
    fn(NonZeroU32) -> BenchIterator,
    fn(NonZeroU32) -> BenchIterator,
)] = &[
    add_test!(funnel, async_channel),
    add_test!(funnel, flume),
    add_test!(funnel, tachyonix),
    add_test!(funnel, postage_mpsc),
    add_test!(funnel, tokio_mpsc),
    add_test!(pinball, async_channel),
    add_test!(pinball, flume),
    add_test!(pinball, tachyonix),
    add_test!(pinball, postage_mpsc),
    add_test!(pinball, tokio_mpsc),
];

pub struct BenchResult {
    label: String,
    parameter: String,
    throughput: Vec<f64>,
}
impl BenchResult {
    pub fn new(label: String, parameter: String, throughput: Vec<f64>) -> Self {
        Self {
            label,
            parameter,
            throughput,
        }
    }
}

type BenchIterator = Box<dyn Iterator<Item = BenchResult>>;

enum ExecutorId {
    Tokio,
    AsyncStd,
    SmolScale,
    Asynchronix,
}
impl ExecutorId {
    fn new(name: &str) -> Result<Self, ()> {
        match name {
            "tokio" => Ok(ExecutorId::Tokio),
            "async-std" => Ok(ExecutorId::AsyncStd),
            "smolscale" => Ok(ExecutorId::SmolScale),
            "asynchronix" => Ok(ExecutorId::Asynchronix),
            _ => Err(()),
        }
    }
    fn name(&self) -> &'static str {
        match self {
            ExecutorId::Tokio => "tokio",
            ExecutorId::AsyncStd => "async-std",
            ExecutorId::SmolScale => "smolscale",
            ExecutorId::Asynchronix => "asynchronix",
        }
    }
}

struct BenchArgs {
    bench_names: Vec<String>,
    executor: ExecutorId,
    samples: NonZeroU32,
    output: Option<OsString>,
}

fn parse_args() -> Result<Option<BenchArgs>, lexopt::Error> {
    let mut samples = NonZeroU32::new(1).unwrap();
    let mut executor = ExecutorId::Tokio;
    let mut bench_names = Vec::new();
    let mut output = None;

    let mut parser = lexopt::Parser::from_env();
    while let Some(arg) = parser.next()? {
        match arg {
            Short('h') | Long("help") => {
                println!("{}", HELP_MESSAGE);

                return Ok(None);
            }
            Short('l') | Long("list") => {
                for (group, item, _, _, _, _) in BENCHES {
                    println!("    {}-{}", group, item)
                }

                return Ok(None);
            }
            Short('s') | Long("samples") => {
                samples = parser.value()?.parse()?;
            }
            Short('o') | Long("output") => {
                output = Some(parser.value()?);
            }
            Short('e') | Long("exec") => {
                let val = parser.value()?;
                executor = ExecutorId::new(val.clone().into_string()?.as_ref()).map_err(|_| {
                    lexopt::Error::UnexpectedValue {
                        option: "exec".into(),
                        value: val,
                    }
                })?;
            }
            Value(val) => {
                bench_names.push(val.into_string()?);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Some(BenchArgs {
        bench_names,
        executor,
        samples,
        output,
    }))
}

fn main() -> Result<(), lexopt::Error> {
    #[allow(clippy::type_complexity)]
    let mut benches: BTreeMap<
        &'static str,
        BTreeMap<&'static str, fn(NonZeroU32) -> Box<dyn Iterator<Item = BenchResult>>>,
    > = BTreeMap::new();

    let BenchArgs {
        bench_names,
        executor,
        samples,
        output,
    } = match parse_args()? {
        None => return Ok(()),
        Some(args) => args,
    };

    if bench_names.is_empty() {
        for (group, item, tokio_bench, async_std_bench, smolscale_bench, asynchronix_bench) in
            BENCHES
        {
            let bench = match executor {
                ExecutorId::Tokio => tokio_bench,
                ExecutorId::AsyncStd => async_std_bench,
                ExecutorId::SmolScale => smolscale_bench,
                ExecutorId::Asynchronix => asynchronix_bench,
            };
            benches
                .entry(*group)
                .or_insert(BTreeMap::new())
                .insert(*item, *bench);
        }
    } else {
        for (group, item, tokio_bench, async_std_bench, smolscale_bench, asynchronix_bench) in
            BENCHES
        {
            let bench_name = format!("{}-{}", group, item);

            for name in &bench_names {
                if bench_names.is_empty() || bench_name.contains(name) {
                    let bench = match executor {
                        ExecutorId::Tokio => tokio_bench,
                        ExecutorId::AsyncStd => async_std_bench,
                        ExecutorId::SmolScale => smolscale_bench,
                        ExecutorId::Asynchronix => asynchronix_bench,
                    };
                    benches
                        .entry(*group)
                        .or_insert(BTreeMap::new())
                        .insert(*item, *bench);
                }
            }
        }
    }

    if benches.is_empty() {
        println!("No matching benches found");

        return Ok(());
    }

    let mut output = output
        .map(|filename| {
            File::create(filename.clone())
                .map_err(|_| format!("Could not open file <{}>", filename.to_str().unwrap()))
        })
        .transpose()?;

    for (group, benches) in benches {
        println!(
            "Running benchmark '{}' with the {} runtime.",
            group,
            executor.name()
        );
        if samples.get() != 1 {
            println!("All results are averaged over {} runs.", samples);
        }

        // Only used when saving to file.
        let mut column_headers = Vec::new();
        let mut parameter_column = Vec::new();
        let mut columns = Vec::new();

        for (bench_id, (name, bench)) in benches.into_iter().enumerate() {
            println!("    {}:", name);
            let mut data_column = Vec::new();

            for (
                parameter_id,
                BenchResult {
                    label,
                    parameter,
                    throughput,
                },
            ) in bench(samples).into_iter().enumerate()
            {
                assert!(!throughput.is_empty());

                let mean = throughput.iter().fold(0f64, |acc, s| acc + s) / throughput.len() as f64;

                if output.is_some() {
                    if bench_id == 0 && parameter_id == 0 {
                        column_headers.push(label.clone());
                    }
                    if bench_id == 0 {
                        parameter_column.push(parameter.clone());
                    }
                    data_column.push(format!("{:.0}", mean));
                }

                if throughput.len() == 1 {
                    println!(
                        "        {:<20} {:>12.3} msg/µs",
                        format!("{}={}", label, parameter),
                        mean / 1e6
                    );
                } else {
                    let std_dev = (throughput
                        .iter()
                        .fold(0f64, |acc, s| acc + (s - mean) * (s - mean))
                        / throughput.len() as f64)
                        .sqrt();

                    println!(
                        "        {:<20} {:>12.3} msg/µs [±{:.3}]",
                        format!("{}: {}", label, parameter),
                        mean * 1e-6,
                        std_dev * 1e-6
                    );
                }
            }
            if output.is_some() {
                columns.push(data_column);
                column_headers.push(String::from(name));
            }
            println!();
        }

        if let Some(file) = &mut output {
            columns.insert(0, parameter_column);
            writeln!(
                file,
                "# '{}' benchmark with {} runtime",
                group,
                executor.name()
            )
            .unwrap();
            write!(file, "#").unwrap();
            for header in column_headers {
                write!(file, "{:>15} ", header).unwrap();
            }
            writeln!(file).unwrap();
            for row in 0..columns[0].len() {
                for column in &columns {
                    write!(file, " {:>15}", column[row]).unwrap();
                }
                writeln!(file).unwrap();
            }
            writeln!(file).unwrap();
        }
    }

    Ok(())
}
