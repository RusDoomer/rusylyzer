use arrayvec::ArrayVec;
use fxhash::FxHashMap;
use indexmap::IndexMap;
use anyhow::Result;
use serde::Deserialize;
use serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::utility::ConvertU8;

pub type CharacterData = ArrayVec<f64, 60>;
pub type BigramData = FxHashMap<[u8; 2], f64>;
pub type FastBigramData = Vec<f64>;
pub type TrigramData = Vec<([u8; 3], f64)>;

#[derive(Deserialize)]
struct LanguageDataInter {
	pub language: String,
	pub characters: FxHashMap<char, f64>,
	pub bigrams: FxHashMap<String, f64>,
	pub skipgrams: FxHashMap<String, f64>,
	pub skipgrams2: FxHashMap<String, f64>,
	pub skipgrams3: FxHashMap<String, f64>,
	pub trigrams: IndexMap<String, f64>,
	#[serde(skip)]
	pub convert_u8: ConvertU8
}

fn get_char_data(data: FxHashMap<char, f64>, con: &mut ConvertU8) -> CharacterData {
	let mut res = CharacterData::new();
	for (c, f) in data.into_iter() {
		con.insert_single(c);
		res.push(f);
	}
	res
}

fn get_bigram_data(data: FxHashMap<String, f64>, con: &mut ConvertU8) -> BigramData {
	let mut res = BigramData::default();
	for (bigram, freq) in data {
		let bv = bigram.chars().collect::<Vec<char>>();
		let bv_u8 = con.to(bv);

		let new_bigram = [bv_u8[0], bv_u8[1]];
		res.insert(new_bigram, freq);
	}
	res
}

fn get_trigram_data(data: IndexMap<String, f64>, con: &mut ConvertU8) -> TrigramData {
	let mut res = TrigramData::new();
	for (trigram, freq) in data {
		let tv = trigram.chars().collect::<Vec<char>>();
		let tv_u8 = con.to(tv);

		if tv_u8[0] != tv_u8[1] && tv_u8[1] != tv_u8[2] {
			let new_trigram = [tv_u8[0], tv_u8[1], tv_u8[2]];
			res.push((new_trigram, freq));
		}
	}
	res
}
pub struct LanguageData {
	pub characters: CharacterData,
	pub bigrams: BigramData,
	pub skipgrams: BigramData,
	pub skipgrams2: BigramData,
	pub skipgrams3: BigramData,
	pub weighted_bigrams: FastBigramData,
	pub trigrams: TrigramData,
	pub language: String,
	pub convert_u8: ConvertU8
}

impl From<LanguageDataInter> for LanguageData {
	fn from(inter: LanguageDataInter) -> Self {
		let mut convert_u8 = inter.convert_u8;
		let characters = get_char_data(inter.characters, &mut convert_u8);

		let bigrams = get_bigram_data(inter.bigrams, &mut convert_u8);
		let skipgrams = get_bigram_data(inter.skipgrams, &mut convert_u8);
		let skipgrams2 = get_bigram_data(inter.skipgrams2, &mut convert_u8);
		let skipgrams3 = get_bigram_data(inter.skipgrams3, &mut convert_u8);

		let weighted_bigrams = FastBigramData::new();

		let trigrams = get_trigram_data(inter.trigrams, &mut convert_u8);

		Self {
			characters, bigrams, skipgrams, skipgrams2, skipgrams3, trigrams,
			weighted_bigrams, language: inter.language, convert_u8
		}
	}
}

impl LanguageData {
	pub fn new(text: &str) -> Result<LanguageData> {
		let data: LanguageDataInter = serde_json::from_str(text)?;
		Ok(LanguageData::from(data))
	}

	pub fn from_file<P>(base_path: P, language: &str) -> Result<LanguageData>
		where P: AsRef<Path> {
		let file_path = base_path.as_ref().join(language.to_lowercase() + ".json");
		let mut file = File::open(file_path)?;
		
		let mut contents = String::new();
		file.read_to_string(&mut contents)?;

		let data: LanguageDataInter = serde_json::from_str(contents.as_str())?;
		let res = LanguageData::from(data);

		Ok(res)
	}
}