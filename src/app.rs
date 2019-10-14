use std::cmp;
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Local};
use clap::{App, AppSettings, Arg, ArgMatches};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;
use stdinout::OrExit;

use crate::{
    BucketConfig, CommonConfig, DepembedsConfig, LossType, ModelType, NGramConfig,
    SimpleVocabConfig, SkipGramConfig, SubwordVocabConfig, Trainer, Vocab, SGD,
};

static DEFAULT_CLAP_SETTINGS: &[AppSettings] = &[
    AppSettings::DontCollapseArgsInUsage,
    AppSettings::UnifiedHelpMessage,
];

// Option constants
static BUCKETS: &str = "buckets";
static CONTEXT: &str = "context";
static CONTEXT_MINCOUNT: &str = "context_mincount";
static CONTEXT_DISCARD: &str = "context_discard";
static DEPENDENCY_DEPTH: &str = "dependency_depth";
static DIMS: &str = "dims";
static DISCARD: &str = "discard";
static EPOCHS: &str = "epochs";
static LR: &str = "lr";
static MINCOUNT: &str = "mincount";
static MINN: &str = "minn";
static MAXN: &str = "maxn";
static MODEL: &str = "model";
static NGRAM_MINCOUNT: &str = "ngram_mincount";
static SUBWORDS: &str = "subwords";
static UNTYPED_DEPS: &str = "untyped";
static NORMALIZE_CONTEXT: &str = "normalize";
static NS: &str = "ns";
static PROJECTIVIZE: &str = "projectivize";
static THREADS: &str = "threads";
static USE_ROOT: &str = "use_root";
static ZIPF_EXPONENT: &str = "zipf";

// Argument constants
static CORPUS: &str = "CORPUS";
static OUTPUT: &str = "OUTPUT";

/// Meta information about training.
///
/// Holds
#[derive(Clone, Serialize)]
pub struct TrainInfo {
    corpus: String,
    output: String,
    n_threads: usize,
    start_datetime: String,
    end_datetime: Option<String>,
}

impl TrainInfo {
    /// Construct new TrainInfo.
    ///
    /// Constructs TrainInfo with `start_datetime` set to the current datetime. `end_datetime` is
    /// set to `None` and can be set through `TrainInfo::set_end`.
    pub fn new(corpus: String, output: String, n_threads: usize) -> Self {
        let start_datetime: DateTime<Local> = Local::now();
        TrainInfo {
            corpus,
            output,
            n_threads,
            start_datetime: start_datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
            end_datetime: None,
        }
    }

    /// Get the corpus path.
    pub fn corpus(&self) -> &str {
        &self.corpus
    }

    /// Get the output file.
    pub fn output(&self) -> &str {
        &self.output
    }

    /// Get the number of threads.
    pub fn n_threads(&self) -> usize {
        self.n_threads
    }

    /// Get the start datetime.
    pub fn start_datetime(&self) -> &str {
        &self.start_datetime
    }

    /// Get the end datetime.
    pub fn end_datetime(&self) -> Option<&str> {
        self.end_datetime.as_ref().map(|s| s.as_str())
    }

    /// Set the end datetime to current datetime.
    pub fn set_end(&mut self) {
        let start_datetime: DateTime<Local> = Local::now();
        self.end_datetime = Some(start_datetime.format("%Y-%m-%d %H:%M:%S").to_string());
    }
}

/// SkipGramApp.
pub struct SkipGramApp {
    train_info: TrainInfo,
    common_config: CommonConfig,
    skipgram_config: SkipGramConfig,
    vocab_config: VocabConfig,
}

impl Default for SkipGramApp {
    fn default() -> Self {
        Self::new()
    }
}

impl SkipGramApp {
    /// Construct new `SkipGramApp`.
    pub fn new() -> Self {
        let matches = build_with_common_opts("ff-train-skipgram")
            .arg(
                Arg::with_name(CONTEXT)
                    .long("context")
                    .value_name("CONTEXT_SIZE")
                    .help("Context size")
                    .takes_value(true)
                    .default_value("10"),
            )
            .arg(
                Arg::with_name(MODEL)
                    .long(MODEL)
                    .value_name("MODEL")
                    .help("Model")
                    .takes_value(true)
                    .possible_values(&["dirgram", "skipgram", "structgram"])
                    .default_value("skipgram"),
            )
            .get_matches();
        let corpus = matches.value_of(CORPUS).unwrap().into();
        let output = matches.value_of(OUTPUT).unwrap().into();
        let n_threads = matches
            .value_of("threads")
            .map(|v| v.parse().or_exit("Cannot parse number of threads", 1))
            .unwrap_or_else(|| cmp::min(num_cpus::get() / 2, 20));
        let train_info = TrainInfo::new(corpus, output, n_threads);
        SkipGramApp {
            train_info,
            common_config: common_config_from_matches(&matches),
            skipgram_config: Self::skipgram_config_from_matches(&matches),
            vocab_config: vocab_config_from_matches(&matches),
        }
    }

    /// Get the corpus path.
    pub fn corpus(&self) -> &str {
        self.train_info.corpus.as_str()
    }

    /// Get the output path.
    pub fn output(&self) -> &str {
        self.train_info.output.as_str()
    }

    /// Get the number of threads.
    pub fn n_threads(&self) -> usize {
        self.train_info.n_threads
    }

    /// Get the common config.
    pub fn common_config(&self) -> CommonConfig {
        self.common_config
    }

    /// Get the skipgram config.
    pub fn skipgram_config(&self) -> SkipGramConfig {
        self.skipgram_config
    }

    /// Get the vocab config.
    pub fn vocab_config(&self) -> VocabConfig {
        self.vocab_config
    }

    /// Get the train information.
    pub fn train_info(&self) -> &TrainInfo {
        &self.train_info
    }

    fn skipgram_config_from_matches(matches: &ArgMatches) -> SkipGramConfig {
        let context_size = matches
            .value_of(CONTEXT)
            .map(|v| v.parse().or_exit("Cannot parse context size", 1))
            .unwrap();
        let model = matches
            .value_of(MODEL)
            .map(|v| ModelType::try_from_str(v).or_exit("Cannot parse model type", 1))
            .unwrap();

        SkipGramConfig {
            context_size,
            model,
        }
    }
}

/// DepembedsApp.
pub struct DepembedsApp {
    train_info: TrainInfo,
    common_config: CommonConfig,
    depembeds_config: DepembedsConfig,
    input_vocab_config: VocabConfig,
    output_vocab_config: SimpleVocabConfig,
}

impl Default for DepembedsApp {
    fn default() -> Self {
        Self::new()
    }
}

impl DepembedsApp {
    /// Construct a new `DepembedsApp`.
    pub fn new() -> Self {
        let matches =
            Self::add_depembeds_opts(build_with_common_opts("ff-train-deps")).get_matches();
        let corpus = matches.value_of(CORPUS).unwrap().into();
        let output = matches.value_of(OUTPUT).unwrap().into();
        let n_threads = matches
            .value_of("threads")
            .map(|v| v.parse().or_exit("Cannot parse number of threads", 1))
            .unwrap_or_else(|| cmp::min(num_cpus::get() / 2, 20));

        let discard_threshold = matches
            .value_of(CONTEXT_DISCARD)
            .map(|v| v.parse().or_exit("Cannot parse discard threshold", 1))
            .unwrap();
        let min_count = matches
            .value_of(CONTEXT_MINCOUNT)
            .map(|v| v.parse().or_exit("Cannot parse mincount", 1))
            .unwrap();

        let output_vocab_config = SimpleVocabConfig {
            min_count,
            discard_threshold,
        };
        let train_info = TrainInfo::new(corpus, output, n_threads);

        DepembedsApp {
            train_info,
            common_config: common_config_from_matches(&matches),
            depembeds_config: Self::depembeds_config_from_matches(&matches),
            input_vocab_config: vocab_config_from_matches(&matches),
            output_vocab_config,
        }
    }

    /// Get the corpus path.
    pub fn corpus(&self) -> &str {
        self.train_info.corpus.as_str()
    }

    /// Get the output path.
    pub fn output(&self) -> &str {
        self.train_info.output.as_str()
    }

    /// Get the number of threads.
    pub fn n_threads(&self) -> usize {
        self.train_info.n_threads
    }

    /// Get the common config.
    pub fn common_config(&self) -> CommonConfig {
        self.common_config
    }

    /// Get the depembeds config.
    pub fn depembeds_config(&self) -> DepembedsConfig {
        self.depembeds_config
    }

    /// Get the input vocab config.
    pub fn input_vocab_config(&self) -> VocabConfig {
        self.input_vocab_config
    }

    /// Get the output vocab config.
    pub fn output_vocab_config(&self) -> SimpleVocabConfig {
        self.output_vocab_config
    }

    /// Get the train information.
    pub fn train_info(&self) -> &TrainInfo {
        &self.train_info
    }

    fn add_depembeds_opts<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
        app.arg(
            Arg::with_name(CONTEXT_DISCARD)
                .long("context_discard")
                .value_name("CONTEXT_THRESHOLD")
                .help("Context discard threshold")
                .takes_value(true)
                .default_value("1e-4"),
        )
        .arg(
            Arg::with_name(CONTEXT_MINCOUNT)
                .long("context_mincount")
                .value_name("CONTEXT_FREQ")
                .help("Context mincount")
                .takes_value(true)
                .default_value("5"),
        )
        .arg(
            Arg::with_name(DEPENDENCY_DEPTH)
                .long("dependency_depth")
                .value_name("DEPENDENCY_DEPTH")
                .help("Dependency depth")
                .takes_value(true)
                .default_value("1"),
        )
        .arg(
            Arg::with_name(UNTYPED_DEPS)
                .long("untyped_deps")
                .help("Don't use dependency relation labels."),
        )
        .arg(
            Arg::with_name(NORMALIZE_CONTEXT)
                .long("normalize_context")
                .help("Normalize contexts"),
        )
        .arg(
            Arg::with_name(PROJECTIVIZE)
                .long("projectivize")
                .help("Projectivize dependency graphs before training."),
        )
        .arg(
            Arg::with_name(USE_ROOT)
                .long("use_root")
                .help("Use root when extracting dependency contexts."),
        )
    }

    fn depembeds_config_from_matches(matches: &ArgMatches) -> DepembedsConfig {
        let depth = matches
            .value_of(DEPENDENCY_DEPTH)
            .map(|v| v.parse().or_exit("Cannot parse dependency depth", 1))
            .unwrap();
        let untyped = matches.is_present(UNTYPED_DEPS);
        let normalize = matches.is_present(NORMALIZE_CONTEXT);
        let projectivize = matches.is_present(PROJECTIVIZE);
        let use_root = matches.is_present(USE_ROOT);
        DepembedsConfig {
            depth,
            untyped,
            normalize,
            projectivize,
            use_root,
        }
    }
}

fn build_with_common_opts<'a, 'b>(name: &str) -> App<'a, 'b> {
    let version = if let Some(git_desc) = option_env!("MAYBE_FINALFRONTIER_GIT_DESC") {
        git_desc
    } else {
        env!("CARGO_PKG_VERSION")
    };
    App::new(name)
        .settings(DEFAULT_CLAP_SETTINGS)
        .version(version)
        .arg(
            Arg::with_name(BUCKETS)
                .long("buckets")
                .value_name("EXP")
                .help("Number of buckets: 2^EXP")
                .takes_value(true)
                .default_value("21"),
        )
        .arg(
            Arg::with_name(DIMS)
                .long("dims")
                .value_name("DIMENSIONS")
                .help("Embedding dimensionality")
                .takes_value(true)
                .default_value("300"),
        )
        .arg(
            Arg::with_name(DISCARD)
                .long("discard")
                .value_name("THRESHOLD")
                .help("Discard threshold")
                .takes_value(true)
                .default_value("1e-4"),
        )
        .arg(
            Arg::with_name(EPOCHS)
                .long("epochs")
                .value_name("N")
                .help("Number of epochs")
                .takes_value(true)
                .default_value("15"),
        )
        .arg(
            Arg::with_name(LR)
                .long("lr")
                .value_name("LEARNING_RATE")
                .help("Initial learning rate")
                .takes_value(true)
                .default_value("0.05"),
        )
        .arg(
            Arg::with_name(MINCOUNT)
                .long("mincount")
                .value_name("FREQ")
                .help("Minimum token frequency")
                .takes_value(true)
                .default_value("5"),
        )
        .arg(
            Arg::with_name(MINN)
                .long("minn")
                .value_name("LEN")
                .help("Minimum ngram length")
                .takes_value(true)
                .default_value("3"),
        )
        .arg(
            Arg::with_name(MAXN)
                .long("maxn")
                .value_name("LEN")
                .help("Maximum ngram length")
                .takes_value(true)
                .default_value("6"),
        )
        .arg(
            Arg::with_name(SUBWORDS)
                .long("subwords")
                .takes_value(true)
                .value_name("SUBWORDS")
                .possible_values(&["buckets", "ngrams", "none"])
                .default_value("buckets")
                .help("What kind of subwords to use."),
        )
        .arg(
            Arg::with_name(NGRAM_MINCOUNT)
                .long("ngram_mincount")
                .value_name("FREQ")
                .help("Minimum ngram frequency.")
                .takes_value(true)
                .default_value("5"),
        )
        .arg(
            Arg::with_name(NS)
                .long("ns")
                .value_name("FREQ")
                .help("Negative samples per word")
                .takes_value(true)
                .default_value("5"),
        )
        .arg(
            Arg::with_name(THREADS)
                .long("threads")
                .value_name("N")
                .help("Number of threads (default: min(logical_cpus / 2, 20))")
                .takes_value(true),
        )
        .arg(
            Arg::with_name(ZIPF_EXPONENT)
                .long("zipf")
                .value_name("EXP")
                .help("Exponent Zipf distribution for negative sampling")
                .takes_value(true)
                .default_value("0.5"),
        )
        .arg(
            Arg::with_name(CORPUS)
                .help("Tokenized corpus")
                .index(1)
                .required(true),
        )
        .arg(
            Arg::with_name(OUTPUT)
                .help("Embeddings output")
                .index(2)
                .required(true),
        )
}

/// Construct `CommonConfig` from `matches`.
fn common_config_from_matches(matches: &ArgMatches) -> CommonConfig {
    let dims = matches
        .value_of(DIMS)
        .map(|v| v.parse().or_exit("Cannot parse dimensionality", 1))
        .unwrap();
    let epochs = matches
        .value_of(EPOCHS)
        .map(|v| v.parse().or_exit("Cannot parse number of epochs", 1))
        .unwrap();
    let lr = matches
        .value_of(LR)
        .map(|v| v.parse().or_exit("Cannot parse learning rate", 1))
        .unwrap();
    let negative_samples = matches
        .value_of(NS)
        .map(|v| {
            v.parse()
                .or_exit("Cannot parse number of negative samples", 1)
        })
        .unwrap();
    let zipf_exponent = matches
        .value_of(ZIPF_EXPONENT)
        .map(|v| {
            v.parse()
                .or_exit("Cannot parse exponent zipf distribution", 1)
        })
        .unwrap();

    CommonConfig {
        loss: LossType::LogisticNegativeSampling,
        dims,
        epochs,
        lr,
        negative_samples,
        zipf_exponent,
    }
}

#[derive(Copy, Clone)]
pub enum VocabConfig {
    SubwordVocab(SubwordVocabConfig<BucketConfig>),
    NGramVocab(SubwordVocabConfig<NGramConfig>),
    SimpleVocab(SimpleVocabConfig),
}

/// Construct `SubwordVocabConfig` from `matches`.
fn vocab_config_from_matches(matches: &ArgMatches) -> VocabConfig {
    let discard_threshold = matches
        .value_of(DISCARD)
        .map(|v| v.parse().or_exit("Cannot parse discard threshold", 1))
        .unwrap();
    let min_count = matches
        .value_of(MINCOUNT)
        .map(|v| v.parse().or_exit("Cannot parse mincount", 1))
        .unwrap();
    let min_n = matches
        .value_of(MINN)
        .map(|v| v.parse().or_exit("Cannot parse minimum n-gram length", 1))
        .unwrap();
    let max_n = matches
        .value_of(MAXN)
        .map(|v| v.parse().or_exit("Cannot parse maximum n-gram length", 1))
        .unwrap();
    match matches.value_of(SUBWORDS).unwrap() {
        "buckets" => {
            let buckets_exp = matches
                .value_of(BUCKETS)
                .map(|v| v.parse().or_exit("Cannot parse bucket exponent", 1))
                .unwrap();
            VocabConfig::SubwordVocab(SubwordVocabConfig {
                discard_threshold,
                min_count,
                max_n,
                min_n,
                indexer: BucketConfig { buckets_exp },
            })
        }
        "ngrams" => {
            let min_ngram_count = matches
                .value_of(NGRAM_MINCOUNT)
                .map(|v| v.parse().or_exit("Cannot parse bucket exponent", 1))
                .unwrap();
            VocabConfig::NGramVocab(SubwordVocabConfig {
                discard_threshold,
                min_count,
                max_n,
                min_n,
                indexer: NGramConfig { min_ngram_count },
            })
        }
        "none" => VocabConfig::SimpleVocab(SimpleVocabConfig {
            min_count,
            discard_threshold,
        }),
        // unreachable as long as possible values in clap are in sync with this `VocabConfig`'s
        // variants
        s => unreachable!(format!("Unhandled vocab type: {}", s)),
    }
}

pub fn show_progress<T, V>(config: &CommonConfig, sgd: &SGD<T>, update_interval: Duration)
where
    T: Trainer<InputVocab = V>,
    V: Vocab,
{
    let n_tokens = sgd.model().input_vocab().n_types();

    let pb = ProgressBar::new(u64::from(config.epochs) * n_tokens as u64);
    pb.set_style(
        ProgressStyle::default_bar().template("{bar:30} {percent}% {msg} ETA: {eta_precise}"),
    );

    while sgd.n_tokens_processed() < n_tokens * config.epochs as usize {
        let lr = (1.0
            - (sgd.n_tokens_processed() as f32 / (config.epochs as usize * n_tokens) as f32))
            * config.lr;

        pb.set_position(sgd.n_tokens_processed() as u64);
        pb.set_message(&format!(
            "loss: {:.*} lr: {:.*}",
            5,
            sgd.train_loss(),
            5,
            lr
        ));

        thread::sleep(update_interval);
    }

    pb.finish();
}