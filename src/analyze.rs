use std::io::{Write, Read};
use std::fs::File;

use crate::language_data::*;
use crate::language_data::LanguageData;
use crate::analysis::{EFFORT_MAP, get_sfb_indices};
use crate::trigram_patterns::*;
use crate::generate::{Layout, BasicLayout};

use anyhow::Result;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Clone, Default)]
pub struct TrigramStats {
	alternates: f64,
	alternates_sfs: f64,
	inrolls: f64,
	outrolls: f64,
	onehands: f64,
	redirects: f64,
	bad_redirects: f64,
	other: f64,
	invalid: f64
}

impl std::fmt::Display for TrigramStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
"Inrolls: {:.3}%
Outrolls: {:.3}% 
Total Rolls: {:.3}%
Onehands: {:.3}%\n
Alternates: {:.3}%
Alternates (sfs): {:.3}%
Total Alternates: {:.3}%\n
Redirects: {:.3}%
Bad Redirects: {:.3}%
Total Redirects: {:.3}%",
			self.inrolls*100.0,
			self.outrolls*100.0,
			(self.inrolls + self.outrolls)*100.0,
			self.onehands*100.0,
			self.alternates*100.0,
			self.alternates_sfs*100.0,
			(self.alternates + self.alternates_sfs)*100.0,
			self.redirects*100.0,
			self.bad_redirects*100.0,
			(self.redirects + self.bad_redirects)*100.0
		)
	}
}

impl std::fmt::Debug for TrigramStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Inrolls: {:.3}%\n
			Outrolls: {:.3}%\n
			Total Rolls: {:.3}%\n
			Onehands: {:.3}%\n\n\
			Alternates: {:.3}%\n
			Alternates (sfs): {:.3}%\n
			Total Alternates: {:.3}%\n\n
			Redirects: {:.3}%\n\
			Bad Redirects: {:.3}%\n
			Total Redirects: {:.3}%\n\n
			Other: {:.3}%\n
			Invalid: {:.3}%",
			self.inrolls*100.0,
			self.outrolls*100.0,
			(self.inrolls + self.outrolls)*100.0,
			self.onehands*100.0,
			self.alternates*100.0,
			self.alternates_sfs*100.0,
			(self.alternates + self.alternates_sfs)*100.0,
			self.redirects*100.0,
			self.bad_redirects*100.0,
			(self.redirects + self.bad_redirects)*100.0,
			self.other*100.0,
			self.invalid*100.0
		)
	}
}

#[derive(Clone)]
struct LayoutStats {
	sfb: f64,
	dsfb: f64,
	trigram_stats: TrigramStats,
	// finger_speed: [f64; 8]
}

impl std::fmt::Display for LayoutStats {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// const BASE: String = String::new();
		// let mut fspeed: [String; 4] = [BASE; 4];
		// for i in 0..4 {
		// 	fspeed[i] = format!("{:.1} {:.1}", self.finger_speed[i], self.finger_speed[7-i]);
		// }
		// let fspeed_print = fspeed.join("\n");
		write!(
			f, "Sfb:  {:.3}%\nDsfb: {:.3}%\n\n{}",
			self.sfb * 100.0, self.dsfb * 100.0, self.trigram_stats
		)
	}
}

#[derive(Deserialize)]
pub struct Defaults {
	pub language: String
}

#[derive(Deserialize, Clone)]
pub struct Weights {
	heatmap: [f64; 2],
	sfb: f64,
	dsfb: f64,
	inrolls: f64,
	outrolls: f64,
	onehands: f64,
	alternates: f64,
	alternates_sfs: f64,
	redirects: f64,
	bad_redirects: f64,
}

#[derive(Deserialize)]
pub struct Config {
	pub defaults: Defaults,
	pub weights: Weights
}

impl Config {
	pub fn new() -> Self {
		let mut f = File::open("config.toml")
			.expect("The config.toml is missing! Help!");
			
		let mut buf = Vec::new();
		f.read_to_end(&mut buf)
			.expect("Failed to read config.toml for some reason");

		toml::from_slice(buf.as_ref())
			.expect("Failed to parse config.toml. Values might be missing.")
	}

	pub fn default() -> Self {
		Self {
			defaults: Defaults {
				language: "english".to_string()
			},
			weights: Weights {
				heatmap: [1.4, 0.6],
				sfb: 15.0,
				dsfb: 2.5,
				inrolls: 0.6,
				outrolls: 0.4,
				onehands: 0.5,
				alternates: 0.5,
				alternates_sfs: 0.25,
				redirects: 0.5,
				bad_redirects: 4.5
			}
		}
	}
}

pub struct LayoutAnalysis {
	language: String,
	layouts: IndexMap<String, BasicLayout>,
	language_data: LanguageData,
	sfb_indices: [(usize, usize); 48],
	weights: Weights
	// col_distance: [f64; 6],
	// index_distance: [f64; 30]
}

impl LayoutAnalysis {
	pub fn new(language: &str, weights: Weights) -> LayoutAnalysis {
		let mut new_analysis = LayoutAnalysis {
			language: language.to_string(),
			layouts: IndexMap::new(),
			language_data: LanguageData::new(language),
			sfb_indices: get_sfb_indices(),
			weights: weights
			// col_distance: [1.0, 2.0, 1.0, 1.0, 2.0, 1.0],
			// index_distance: Self::get_index_distance(1.4)

		};
		new_analysis.layouts = new_analysis.load_layouts().unwrap();
		new_analysis
	}

	fn get_index_distance(lat_penalty: f64) -> [f64; 30] {
		let mut res = [0.0; 30];
		let mut i = 0;
		for y1 in 0..3isize {
			for x1 in 0..2isize {
				for y2 in 0..3isize {
					for x2 in 0..2isize {
						if !(x1 == x2 && y1 == y2) {
							let x_dist = ((x1-x2).abs() as f64)*lat_penalty;
							let y_dist = (y1-y2).abs() as f64;
							let distance = (x_dist.powi(2) + y_dist.powi(2)).sqrt();
							res[i] = distance;
							i += 1;
						}
					}
				}
			}
		}
		res
	}

	fn is_kb_file(entry: &std::fs::DirEntry) -> bool {
		if let Some(ext_os) = entry.path().extension() {
			if let Some(ext) = ext_os.to_str() {
				return ext == "kb"
			}
		}
		false
	}

	fn layout_name(entry: &std::fs::DirEntry) -> Option<String> {
		if let Some(name_os) = entry.path().file_stem() {
			if let Some(name_str) = name_os.to_str() {
				return Some(name_str.to_string())
			}
		}
		None
	}

	fn format_layout_str(layout_str: String) -> String {
		layout_str
			.replace("\n", "")
			.replace("\r", "")
			.replace(" ", "")
	}

	fn load_layouts(&mut self) -> Result<IndexMap<String, BasicLayout>> {
		let mut res: IndexMap<String, BasicLayout> = IndexMap::new();

		let paths = std::fs::read_dir(format!("static/layouts/{}", self.language))?;
		for p in paths {
			if let Ok(entry) = p &&
			Self::is_kb_file(&entry) &&
			let Some(name) = Self::layout_name(&entry) {
				let content = std::fs::read_to_string(entry.path())?;
				let layout_str = Self::format_layout_str(content);

				let mut layout: BasicLayout = BasicLayout::try_from(layout_str.as_str()).unwrap();
				layout.score = self.score(&layout, usize::MAX);

				res.insert(name, layout);
			}
		}
		res.sort_by(|_, a, _, b| {
			b.score.partial_cmp(&a.score).unwrap()
		});
		Ok(res)
	}

	fn get_layout_stats(&self, layout: &BasicLayout) -> LayoutStats {
		let sfb = self.bigram_percent(layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(layout, &self.language_data.skipgrams);
		let trigram_stats = self.trigram_stats(layout, usize::MAX);
		LayoutStats { sfb, dsfb, trigram_stats }
	}

	pub fn rank(&self) {
		for (name, layout) in self.layouts.iter() {
			println!("{:10}{}", format!("{:.3}:", layout.score), name);
		}
	}

	pub fn layout_by_name(&self, name: &str) -> Option<&BasicLayout> {
		self.layouts.get(name)
	}

	pub fn analyze_name(&self, name: &str) {
		let l = match self.layout_by_name(name) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name);
    			return;
  			}
		};
		println!("{}", name);
		self.analyze(&l);
	}

	fn placeholder_name(&self, layout: &BasicLayout) -> Result<String, ()> {
		for i in 1..1000usize {
    		let mut new_name = layout.matrix[10..14].iter().collect::<String>();
			
			new_name.push_str(format!("{}", i).as_str());

			if !self.layouts.contains_key(&new_name) {
				return Ok(new_name);
			}
		}
		Err(())
	}

	pub fn analyze_str(&mut self, layout_str: &str) {
		let layout_str = Self::format_layout_str(layout_str.to_string());
		let layout = BasicLayout::try_from(layout_str.as_str()).unwrap();
		self.analyze(&layout);
	}

	pub fn save(&mut self, mut layout: BasicLayout, name: Option<String>) -> Result<()> {
		let new_name = if let Some(n) = name {
			n.replace(" ", "_")
		} else {
			self.placeholder_name(&layout).unwrap()
		};
		let mut f = std::fs::OpenOptions::new()
			.write(true)
			.create(true)
			.truncate(true)
			.open(format!("static/layouts/{}/{}.kb", self.language, new_name))?;
		
		let layout_formatted = layout.to_string();
		println!("saved {}\n{}", new_name, layout_formatted);
		f.write(layout_formatted.as_bytes()).unwrap();

		layout.score = self.score(&layout, usize::MAX);
		self.layouts.insert(new_name, layout);
		self.layouts.sort_by(|_, a, _, b| {
			b.score.partial_cmp(&a.score).unwrap()
		});

		Ok(())
	}

	pub fn analyze(&self, layout: &BasicLayout) {
		let stats = self.get_layout_stats(layout);
		
		println!("{}\n{}\nScore: {:.3}", layout, stats, layout.score);
		// let x = get_trigram_combinations2();
		// for (i, combination) in x.iter().enumerate() {
		// 	let c1 = i & 0b111;
		// 	let c2 = (i >> 3) & 0b111;
		// 	let c3 = (i >> 6) & 0b111;
		// 	if *combination == TrigramPattern::Other && c1 != c2 && c2 != c3 {
		// 		println!("{:<27} where c1: {}, c2: {}, c3: {}", format!("patterns[{}]: {:?}", i, combination), c1, c2, c3);
		// 	}
		// }
	}

	pub fn compare_name(&self, name1: &str, name2: &str) {
		let l1 = match self.layout_by_name(name1) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name1);
    			return;
  			}
		};
		let l2 = match self.layout_by_name(name2) {
  			Some(layout) => layout,
  			None => {
    			println!("layout {} does not exist!", name2);
    			return;
  			}
		};
		println!("\n{:29}{}", name1, name2);
		for y in 0..3 {
			for (n, layout) in [l1, l2].into_iter().enumerate() {
				for x in 0..10 {
					print!("{} ", layout.matrix[x + 10*y]);
					if x == 4 {
						print!(" ")
					}
				}
				if n == 0 {
					print!("        ")
				}
			}
			println!();
		}
		let s1 = self.get_layout_stats(l1);
		let s2 = self.get_layout_stats(l2);
		let ts1 = s1.trigram_stats;
		let ts2 = s2.trigram_stats;
		// let fs1 = &layouts[0].stats.finger_speed;
		// let fs2 = &layouts[1].stats.finger_speed;
		// const BASE: String = String::new();
		// let mut fspeed: [String; 4] = [BASE; 4];
		// for i in 0..4 {
		// 	fspeed[i] = format!("{:<28} {:.1}, {:.1}",
		// 						format!("{:.1} {:.1}", fs1[i], fs2[7-i]), fs2[i], fs2[7-i]);
		// }
		// let fspeed_print = fspeed.join("\n");
		println!(
			concat!(
			"Sfb:              {:.3}%     Sfb:              {:.3}%\n",
			"Dsfb:             {:.3}%     Dsfb:             {:.3}%\n\n",
			"Inrolls:          {:.2}%     Inrolls:          {:.2}%\n",
			"Outrolls:         {:.2}%     Outrolls:         {:.2}%\n",
			"Total Rolls:      {:.2}%     Total Rolls:      {:.2}%\n",
			"Onehands:         {:.3}%     Onehands:         {:.3}%\n\n",
			"Alternates:       {:.2}%     Alternates:       {:.2}%\n",
			"Alternates (sfs): {:.2}%      Alternates (sfs): {:.2}%\n",
			"Total Alternates: {:.2}%     Total Alternates: {:.2}%\n\n",
			"Redirects:        {:.3}%     Redirects:        {:.2}%\n",
			"Bad Redirects:    {:.3}%     Bad Redirects:    {:.2}%\n",
			"Total Redirects:  {:.3}%     Total Redirects:  {:.2}%\n\n",
			// "Other:            {:.3}%     Other:            {:.2}%\n",
			// "Invalid:          {:.3}%     Invalid:          {:.2}%\n\n",
			//"{}\n\n",
			"Score:            {:.3}     Score:            {:.3}\n"
		),
			s1.sfb*100.0, s2.sfb*100.0,
			s1.dsfb*100.0, s2.dsfb*100.0,
			ts1.inrolls*100.0, ts2.inrolls*100.0,
			ts1.outrolls*100.0, ts2.outrolls*100.0,
			(ts1.inrolls + ts1.outrolls)*100.0, (ts2.inrolls + ts2.outrolls)*100.0,
			ts1.onehands*100.0, ts2.onehands*100.0,
			ts1.alternates*100.0, ts2.alternates*100.0,
			ts1.alternates_sfs*100.0, ts2.alternates_sfs*100.0,
			(ts1.alternates + ts1.alternates_sfs)*100.0, (ts2.alternates + ts2.alternates_sfs)*100.0,
			ts1.redirects*100.0, ts2.redirects*100.0,
			ts1.bad_redirects*100.0, ts2.bad_redirects*100.0,
			(ts1.redirects + ts1.bad_redirects)*100.0, (ts2.redirects + ts2.bad_redirects)*100.0,
			// ts1.other*100.0, ts2.other*100.0,
			// ts1.invalid*100.0, ts2.invalid*100.0,
			//fspeed_print,
			l1.score, l2.score
		);
	}

	pub fn finger_speed(&self, _: &BasicLayout) -> [f64; 8] {
		let res = [0.0; 8];
		res
	}

	pub fn effort(&self, layout: &BasicLayout) -> f64 {
		let mut res: f64 = 0.0;
		for (c, e) in layout.matrix.iter().zip(EFFORT_MAP) {
			res += e * self.language_data.characters.get(c).unwrap_or(&0.0);
		}
		res
	}

	pub fn score(&self, layout: &BasicLayout, trigram_precision: usize) -> f64 {
		let mut score: f64 = 0.0;
		let sfb = self.bigram_percent(layout, &self.language_data.bigrams);
		let dsfb = self.bigram_percent(layout, &self.language_data.skipgrams);
		let trigram_data = self.trigram_stats(layout, trigram_precision);
		score -= self.weights.heatmap[0] * (self.effort(layout) - self.weights.heatmap[1]);
		score -= self.weights.sfb * sfb;
		score -= self.weights.dsfb * dsfb;
		score += self.weights.inrolls * trigram_data.inrolls;
		score += self.weights.outrolls * trigram_data.outrolls;
		score += self.weights.onehands * trigram_data.onehands;
		score += self.weights.alternates * trigram_data.alternates;
		score += self.weights.alternates_sfs * trigram_data.alternates_sfs;
		score -= self.weights.redirects * trigram_data.redirects;
		score -= self.weights.bad_redirects * trigram_data.bad_redirects;
		score
	}

	pub fn bigram_percent(&self, layout: &BasicLayout, data: &BigramData) -> f64 {
		let mut res = 0.0;
		for (i1, i2) in self.sfb_indices {
			let c1 = layout.matrix[i1];
			let c2 = layout.matrix[i2];
			res += data.get(&[c1, c2]).unwrap_or(&0.0);
			res += data.get(&[c2, c1]).unwrap_or(&0.0);
		}
		res
	}

	pub fn trigram_stats(&self, layout: &BasicLayout, trigram_precision: usize) -> TrigramStats {
		let mut freqs = TrigramStats::default();
		for (i, (trigram, freq)) in self.language_data.trigrams.iter().enumerate() {
			if i == trigram_precision {
				return freqs
			}
			match layout.get_trigram_pattern(trigram) {
				TrigramPattern::Alternate => freqs.alternates += freq,
				TrigramPattern::AlternateSfs => freqs.alternates_sfs += freq,
				TrigramPattern::Inroll => freqs.inrolls += freq,
				TrigramPattern::Outroll => freqs.outrolls += freq,
				TrigramPattern::Onehand => freqs.onehands += freq,
				TrigramPattern::Redirect => freqs.redirects += freq,
				TrigramPattern::BadRedirect => freqs.bad_redirects += freq,
				TrigramPattern::Other => freqs.other += freq,
				TrigramPattern::Invalid => freqs.invalid += freq
			}
		}
		freqs
	}
}

