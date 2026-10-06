use log::debug;

use crate::enums::{FilterMode, PokemonInfo, PokemonType, Region, SortDirection, SortKey};
use std::collections::{HashMap, HashSet};
use std::format;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeOriginator {
    Height,
    Weight,
}

impl RangeOriginator {
    pub fn abs_bounds(self) -> (f32, f32) {
        match self {
            RangeOriginator::Height => (0.0, 99.99),
            RangeOriginator::Weight => (0.0, 999.9),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            RangeOriginator::Height => "Height",
            RangeOriginator::Weight => "Weight",
        }
    }

    // Values are stored as feet.inches-as-hundredths (e.g. 1.09 == 1'09"),
    // which is also why 0.01 is the natural minimum gap.
    pub fn format(self, value: f32) -> String {
        match self {
            RangeOriginator::Height => {
                let feet = value.trunc() as i32;
                let inches = (value.fract() * 100.0).round() as i32;

                format!("{feet}'{inches:02}\"")
            }
            RangeOriginator::Weight => {
                let lb = value.trunc() as i32;
                let oz = (value.fract() * 100.0).round() as i32;
                format!("{lb}lb {oz:02}oz")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCriteria {
    pub search: String,
    pub regions: Vec<Region>,
    pub types: Vec<PokemonType>,
    pub filter_mode: FilterMode,
    pub sort_key: SortKey,
    pub sort_order: SortDirection,
    pub height_lower: f32,
    pub height_upper: f32,
    pub weight_lower: f32,
    pub weight_upper: f32,
}

impl Default for FilterCriteria {
    fn default() -> Self {
        Self {
            search: String::new(),
            regions: Vec::from(Region::ALL),
            types: Vec::from(PokemonType::ALL),
            filter_mode: FilterMode::Any,
            sort_key: SortKey::Numerical,
            sort_order: SortDirection::Ascending,
            height_lower: 0.0,
            height_upper: RangeOriginator::Height.abs_bounds().1,
            weight_lower: 0.0,
            weight_upper: RangeOriginator::Weight.abs_bounds().1,
        }
    }
}

impl FilterCriteria {
    fn is_height_active(&self) -> bool {
        self.height_lower > 0.0 || self.height_upper < f32::MAX
    }

    fn is_weight_active(&self) -> bool {
        self.weight_lower > 0.0 || self.weight_upper < f32::MAX
    }

    /// True when no constraint is active — everything matches.
    pub fn is_all_selected(&self) -> bool {
        self.regions == Vec::from(Region::ALL) && self.types == Vec::from(PokemonType::ALL)
    }

    /// Whether `name`/`info` satisfies this filter. Each category below
    /// contributes at most one bool to `active_results`, and only if that
    /// category has an active constraint — an unset category never forces
    /// a match *or* an exclusion. `filter_all` then ANDs vs ORs the active
    /// categories together.
    /// Do not check for base form if filtering from an owned pokemon
    pub fn matches(&self, name: &str, info: &PokemonInfo, check_base: bool) -> bool {
        let mut active_results = Vec::with_capacity(5);

        if name.contains("mega ") || (check_base && info.base.is_some_and(|base| !base)) {
            return false;
        }

        if !self.search.trim().is_empty() {
            let query = self.search.to_lowercase();
            let name_match = name.to_lowercase().contains(&query);
            let display_match = info
                .display_name
                .as_deref()
                .is_some_and(|d| d.to_lowercase().contains(&query));
            let matches = name_match || display_match;
            if matches {
                active_results.push(true);
            } else {
                return false;
            }
        }

        if !self.regions.is_empty() {
            if self.filter_mode == FilterMode::Any {
                active_results.push(
                    info.region
                        .as_ref()
                        .map_or(false, |region| self.regions.contains(region)),
                );
            } else if info
                .region
                .is_some_and(|region| self.regions.contains(&region))
            {
                active_results.push(true);
            } else {
                return false;
            }
        }

        if !self.types.is_empty() {
            if self.filter_mode == FilterMode::Any {
                active_results.push(info.types.iter().any(|t| self.types.contains(t)));
            } else if info.types.len() == self.types.len() {
                let set1: HashSet<&PokemonType> = info.types.iter().collect();
                let set2: HashSet<&PokemonType> = self.types.iter().collect();

                if set1 == set2 {
                    debug!("Got exact type matches {:?} for {:?}", set1, name);
                }

                if set1 == set2 {
                    active_results.push(true);
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }

        if self.is_height_active() {
            let h = info.height.metric;
            let fits = h >= self.height_lower && h <= self.height_upper;
            if self.filter_mode == FilterMode::Any {
                active_results.push(fits);
            } else if fits {
                active_results.push(true);
            } else {
                return false;
            }
        }

        if self.is_weight_active() {
            let w = info.weight.metric;
            let fits = w >= self.weight_lower && w <= self.weight_upper;
            if self.filter_mode == FilterMode::Any {
                active_results.push(fits);
            } else if fits {
                active_results.push(true);
            } else {
                return false;
            }
        }

        if active_results.is_empty() {
            return true;
        }

        active_results.into_iter().all(|matched| matched)
    }

    /// Sort key for a name, honoring `is_alphabetical`. Ascending/descending
    /// is applied separately (reverse the sorted Vec) since `sort_by_cached_key`
    /// takes no comparator — this also means the key is computed once per
    /// element instead of on every comparison.
    pub fn sort_key(&self, name: &str, pokemon_data: &HashMap<String, PokemonInfo>) -> Sorted {
        if self.sort_key == SortKey::Alphabetical {
            let display = pokemon_data
                .get(name)
                .and_then(|i| i.display_name.as_deref())
                .unwrap_or(name);
            Sorted::Alpha(display.to_lowercase())
        } else {
            let num = pokemon_data
                .get(name)
                .and_then(|i| i.number.parse::<u32>().ok())
                .unwrap_or(u32::MAX);
            Sorted::Numeric(num, name.to_string()) // name as tie-break for determinism
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sorted {
    Alpha(String),
    Numeric(u32, String),
}
