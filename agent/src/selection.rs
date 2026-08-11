use std::fmt::Display;
use std::num::NonZeroU8;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

use crate::ScoredAction;

const ACTION_SELECTOR_SEED_DOMAIN: u64 = 0x6a09_e667_f3bc_c909;

/// Validated rank-based Softmax selection parameters.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct RankSoftmaxPolicy {
    top_k: NonZeroU8,
    temperature: f32,
}

impl RankSoftmaxPolicy {
    pub fn new(top_k: NonZeroU8, temperature: f32) -> Result<Self, ActionSelectionError> {
        if !temperature.is_finite() || temperature <= 0.0 {
            return Err(ActionSelectionError::InvalidTemperature(temperature));
        }
        Ok(Self { top_k, temperature })
    }

    pub const fn top_k(self) -> NonZeroU8 {
        self.top_k
    }

    pub const fn temperature(self) -> f32 {
        self.temperature
    }
}

impl Eq for RankSoftmaxPolicy {}

/// Score-sensitive Softmax parameters with a deterministic best-action gap.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ScoreSoftmaxPolicy {
    top_k: NonZeroU8,
    temperature: f32,
    deterministic_gap: f32,
}

impl ScoreSoftmaxPolicy {
    pub fn new(
        top_k: NonZeroU8, temperature: f32, deterministic_gap: f32,
    ) -> Result<Self, ActionSelectionError> {
        if !temperature.is_finite() || temperature <= 0.0 {
            return Err(ActionSelectionError::InvalidTemperature(temperature));
        }
        if !deterministic_gap.is_finite() || deterministic_gap <= 0.0 {
            return Err(ActionSelectionError::InvalidDeterministicGap(deterministic_gap));
        }
        Ok(Self { top_k, temperature, deterministic_gap })
    }

    pub const fn top_k(self) -> NonZeroU8 {
        self.top_k
    }

    pub const fn temperature(self) -> f32 {
        self.temperature
    }

    pub const fn deterministic_gap(self) -> f32 {
        self.deterministic_gap
    }
}

impl Eq for ScoreSoftmaxPolicy {}

/// How an analyzed candidate list is converted into one executed action.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ActionSelectionPolicy {
    /// Always execute the first, highest-ranked candidate.
    #[default]
    Best,
    /// Sample candidates by rank with `P(rank = i) proportional to exp(-i / temperature)`.
    RankSoftmax(RankSoftmaxPolicy),
    /// Sample close candidates from their score gaps and keep clear best moves deterministic.
    ScoreSoftmax(ScoreSoftmaxPolicy),
}

impl ActionSelectionPolicy {
    pub fn rank_softmax(top_k: NonZeroU8, temperature: f32) -> Result<Self, ActionSelectionError> {
        let policy = RankSoftmaxPolicy::new(top_k, temperature)?;
        Ok(Self::RankSoftmax(policy))
    }

    pub fn score_softmax(
        top_k: NonZeroU8, temperature: f32, deterministic_gap: f32,
    ) -> Result<Self, ActionSelectionError> {
        let policy = ScoreSoftmaxPolicy::new(top_k, temperature, deterministic_gap)?;
        Ok(Self::ScoreSoftmax(policy))
    }

    /// Conservative built-in rank policy retained for persisted configurations.
    pub fn standard_rank_softmax() -> Self {
        Self::rank_softmax(NonZeroU8::new(4).expect("standard top_k must be nonzero"), 0.5)
            .expect("standard rank Softmax policy must be valid")
    }

    /// Score-aware placement variety: close moves are sampled, while a gap of
    /// five hundredths in the public score range keeps the best move deterministic.
    pub fn standard_score_softmax() -> Self {
        Self::score_softmax(NonZeroU8::new(4).expect("standard top_k must be nonzero"), 0.02, 0.05)
            .expect("standard score Softmax policy must be valid")
    }

    pub const fn top_k(self) -> NonZeroU8 {
        match self {
            Self::Best => NonZeroU8::MIN,
            Self::RankSoftmax(policy) => policy.top_k(),
            Self::ScoreSoftmax(policy) => policy.top_k(),
        }
    }
}

/// Invalid action-selection configuration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActionSelectionError {
    InvalidTemperature(f32),
    InvalidDeterministicGap(f32),
}

impl Display for ActionSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTemperature(temperature) => write!(
                formatter,
                "Softmax temperature must be finite and greater than zero, got {temperature}"
            ),
            Self::InvalidDeterministicGap(gap) => write!(
                formatter,
                "score Softmax deterministic gap must be finite and greater than zero, got {gap}"
            ),
        }
    }
}

impl std::error::Error for ActionSelectionError {}

/// Stateful candidate selector used by turn execution.
#[derive(Debug)]
pub struct ActionSelector {
    policy: ActionSelectionPolicy,
    rng: StdRng,
}

impl ActionSelector {
    /// Create a selector using the process random-number source.
    pub fn new(policy: ActionSelectionPolicy) -> Self {
        Self { policy, rng: rand::make_rng() }
    }

    /// Create a reproducible selector for Arena and tests.
    pub fn with_seed(policy: ActionSelectionPolicy, seed: u64) -> Self {
        Self { policy, rng: StdRng::seed_from_u64(seed ^ ACTION_SELECTOR_SEED_DOMAIN) }
    }

    pub const fn policy(&self) -> ActionSelectionPolicy {
        self.policy
    }

    pub const fn top_k(&self) -> NonZeroU8 {
        self.policy.top_k()
    }

    pub(crate) fn select_index(&mut self, candidates: &[ScoredAction]) -> usize {
        let candidate_count = candidates.len();
        debug_assert!(candidate_count > 0, "validated candidate list must be nonempty");
        debug_assert!(
            candidate_count <= usize::from(self.top_k().get()),
            "validated candidate list must not exceed selector top_k"
        );
        if candidate_count == 1 {
            return 0;
        }

        match self.policy {
            ActionSelectionPolicy::Best => 0,
            ActionSelectionPolicy::RankSoftmax(policy) => {
                self.select_rank_softmax(candidate_count, policy)
            },
            ActionSelectionPolicy::ScoreSoftmax(policy) => {
                self.select_score_softmax(candidates, policy)
            },
        }
    }

    fn select_rank_softmax(&mut self, candidate_count: usize, policy: RankSoftmaxPolicy) -> usize {
        let decay = (-1.0 / f64::from(policy.temperature())).exp();
        let total_weight = rank_weights(candidate_count, decay).sum::<f64>();
        let target = self.rng.random::<f64>() * total_weight;
        sample_weighted_index(rank_weights(candidate_count, decay), target, candidate_count)
    }

    fn select_score_softmax(
        &mut self, candidates: &[ScoredAction], policy: ScoreSoftmaxPolicy,
    ) -> usize {
        let best_score = candidates[0].score;
        if best_score - candidates[1].score >= policy.deterministic_gap() {
            return 0;
        }

        let temperature = f64::from(policy.temperature());
        let mut weights = Vec::with_capacity(candidates.len());
        let mut total_weight = 0.0;
        for candidate in candidates {
            let score_gap = f64::from(candidate.score) - f64::from(best_score);
            let weight = (score_gap / temperature).exp();
            weights.push(weight);
            total_weight += weight;
        }
        let target = self.rng.random::<f64>() * total_weight;
        sample_weighted_index(weights, target, candidates.len())
    }
}

impl Default for ActionSelector {
    fn default() -> Self {
        Self::with_seed(ActionSelectionPolicy::Best, 0)
    }
}

fn rank_weights(candidate_count: usize, decay: f64) -> impl Iterator<Item = f64> {
    std::iter::successors(Some(1.0), move |weight| Some(*weight * decay)).take(candidate_count)
}

fn sample_weighted_index(
    weights: impl IntoIterator<Item = f64>, mut target: f64, candidate_count: usize,
) -> usize {
    for (index, weight) in weights.into_iter().enumerate() {
        if target < weight {
            return index;
        }
        target -= weight;
    }
    candidate_count - 1
}
#[cfg(test)]
mod tests {
    use formation_chess_core::action::Action;

    use super::ActionSelectionPolicy;
    use super::ActionSelector;
    use crate::ScoredAction;

    fn candidates(scores: &[f32]) -> Vec<ScoredAction> {
        let mut candidates = Vec::with_capacity(scores.len());
        for (index, &score) in scores.iter().enumerate() {
            candidates.push(ScoredAction {
                action: Action::Resign(u8::try_from(index).expect("test index must fit u8"), 0),
                score,
            });
        }
        candidates
    }

    #[test]
    fn invalid_softmax_temperatures_are_rejected() {
        let top_k = std::num::NonZeroU8::new(4).expect("nonzero top_k");
        for temperature in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            ActionSelectionPolicy::rank_softmax(top_k, temperature)
                .expect_err("invalid rank temperature must be rejected");
            ActionSelectionPolicy::score_softmax(top_k, temperature, 0.05)
                .expect_err("invalid score temperature must be rejected");
        }
    }

    #[test]
    fn invalid_score_gaps_are_rejected() {
        let top_k = std::num::NonZeroU8::new(4).expect("nonzero top_k");
        for gap in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            ActionSelectionPolicy::score_softmax(top_k, 0.02, gap)
                .expect_err("invalid deterministic gap must be rejected");
        }
    }

    #[test]
    fn equal_seeds_reproduce_rank_samples() {
        let policy = ActionSelectionPolicy::standard_rank_softmax();
        let choices = candidates(&[0.4, 0.3, 0.2, 0.1]);
        let mut first = ActionSelector::with_seed(policy, 20260802);
        let mut second = ActionSelector::with_seed(policy, 20260802);
        let mut first_ranks = Vec::new();
        let mut second_ranks = Vec::new();
        for _ in 0 .. 128 {
            first_ranks.push(first.select_index(&choices));
            second_ranks.push(second.select_index(&choices));
        }

        assert_eq!(first_ranks, second_ranks);
    }

    #[test]
    fn standard_rank_policy_favors_better_ranks() {
        let choices = candidates(&[0.4, 0.3, 0.2, 0.1]);
        let mut selector =
            ActionSelector::with_seed(ActionSelectionPolicy::standard_rank_softmax(), 20260802);
        let mut counts = [0_u32; 4];
        for _ in 0 .. 10_000 {
            counts[selector.select_index(&choices)] += 1;
        }

        assert!(counts[0] > counts[1]);
        assert!(counts[1] > counts[2]);
        assert!(counts[2] > counts[3]);
        assert!(counts[0] > 8_000);
    }

    #[test]
    fn score_policy_keeps_exact_min_wins_and_clear_best_moves_deterministic() {
        let policy = ActionSelectionPolicy::standard_score_softmax();
        let mut selector = ActionSelector::with_seed(policy, 20260802);
        let winning = candidates(&[1.0, 0.95, 0.5, 0.0]);
        let clear_best = candidates(&[0.4, 0.3, 0.29, 0.28]);
        for _ in 0 .. 128 {
            assert_eq!(selector.select_index(&winning), 0);
            assert_eq!(selector.select_index(&clear_best), 0);
        }
    }

    #[test]
    fn score_policy_samples_close_moves_by_score_gap() {
        let policy = ActionSelectionPolicy::standard_score_softmax();
        let choices = candidates(&[0.20, 0.19, 0.18, 0.17]);
        let mut selector = ActionSelector::with_seed(policy, 20260802);
        let mut counts = [0_u32; 4];
        for _ in 0 .. 10_000 {
            counts[selector.select_index(&choices)] += 1;
        }

        assert!(counts[0] > counts[1]);
        assert!(counts[1] > counts[2]);
        assert!(counts[2] > counts[3]);
        assert!(counts[3] > 0);
    }
}
