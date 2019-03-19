#[macro_use]
extern crate cfg_if;

extern crate conllx;

extern crate failure;

extern crate fnv;

extern crate hogwild;

#[macro_use]
#[cfg(test)]
extern crate lazy_static;

#[macro_use]
#[cfg(test)]
extern crate maplit;

extern crate ndarray;

extern crate ndarray_rand;

extern crate rand;

extern crate rand_core;

#[cfg(test)]
extern crate rand_xorshift;

extern crate rust2vec;

extern crate serde;

extern crate toml;

extern crate zipf;

mod config;
pub use config::{Config, LossType, ModelType};

mod deps;

mod io;
pub use io::{SentenceIterator, WriteModelBinary, WriteModelText, WriteModelWord2Vec};

pub(crate) mod loss;

pub(crate) mod sampling;

mod sgd;
pub use sgd::SGD;

pub(crate) mod subword;

mod train_model;
pub use train_model::{TrainModel, Trainer};

pub(crate) mod skipgram_trainer;
pub use skipgram_trainer::SkipgramTrainer;

pub(crate) mod util;

pub(crate) mod vec_simd;

mod vocab;
pub use vocab::{CountedType, SubwordVocab, Vocab, VocabBuilder, Word};
