use crate::enums::{FilterMode, PokemonInfo, PokemonType, Region, SortDirection, SortKey};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct FilterCriteria {
    pub search: String,
    pub regions: HashSet<Region>,
    pub types: HashSet<PokemonType>,
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
            regions: HashSet::from(Region::ALL),
            types: HashSet::from(PokemonType::ALL),
            filter_mode: FilterMode::Any,
            sort_key: SortKey::Numerical,
            sort_order: SortDirection::Ascending,
            height_lower: 0.0,
            height_upper: f32::MAX,
            weight_lower: 0.0,
            weight_upper: f32::MAX,
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
    pub fn is_empty(&self) -> bool {
        self.search.trim().is_empty()
            && self.regions.is_empty()
            && self.types.is_empty()
            && !self.is_height_active()
            && !self.is_weight_active()
    }

    /// Whether `name`/`info` satisfies this filter. Each category below
    /// contributes at most one bool to `active_results`, and only if that
    /// category has an active constraint — an unset category never forces
    /// a match *or* an exclusion. `filter_all` then ANDs vs ORs the active
    /// categories together.
    pub fn matches(&self, name: &str, info: &PokemonInfo) -> bool {
        let mut active_results = Vec::with_capacity(5);

        if name.contains("mega ") || info.base.is_some_and(|base| !base) {
            return false;
        }

        if !self.search.trim().is_empty() {
            let query = self.search.to_lowercase();
            let name_match = name.to_lowercase().contains(&query);
            let display_match = info
                .display_name
                .as_deref()
                .is_some_and(|d| d.to_lowercase().contains(&query));
            active_results.push(name_match || display_match);
        }

        if !self.regions.is_empty() {
            active_results.push(
                info.region
                    .as_ref()
                    .map_or(false, |region| self.regions.contains(region)),
            );
        }

        if !self.types.is_empty() {
            active_results.push(info.types.iter().any(|t| self.types.contains(t)));
        }

        if self.is_height_active() {
            let h = info.height.metric;
            active_results.push(h >= self.height_lower && h <= self.height_upper);
        }

        if self.is_weight_active() {
            let w = info.weight.metric;
            active_results.push(w >= self.weight_lower && w <= self.weight_upper);
        }

        if active_results.is_empty() {
            return true;
        }

        if self.filter_mode == FilterMode::All {
            active_results.into_iter().all(|matched| matched)
        } else {
            active_results.into_iter().any(|matched| matched)
        }
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
