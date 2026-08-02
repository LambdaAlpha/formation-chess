use std::fmt::Display;
use std::num::NonZeroU8;

use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;

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

/// How an analyzed candidate list is converted into one executed action.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ActionSelectionPolicy {
    /// Always execute the first, highest-ranked candidate.
    #[default]
    Best,
    /// Sample candidates by rank with `P(rank = i) proportional to exp(-i / temperature)`.
    RankSoftmax(RankSoftmaxPolicy),
}

impl ActionSelectionPolicy {
    pub fn rank_softmax(top_k: NonZeroU8, temperature: f32) -> Result<Self, ActionSelectionError> {
        RankSoftmaxPolicy::new(top_k, temperature).map(Self::RankSoftmax)
    }

    /// Conservative built-in stochastic policy: approximately 86.5%, 11.7%,
    /// 1.6%, and 0.2% for ranks one through four.
    pub fn standard_rank_softmax() -> Self {
        Self::rank_softmax(NonZeroU8::new(4).expect("standard top_k must be nonzero"), 0.5)
            .expect("standard rank Softmax policy must be valid")
    }

    pub const fn top_k(self) -> NonZeroU8 {
        match self {
            Self::Best => NonZeroU8::MIN,
            Self::RankSoftmax(policy) => policy.top_k(),
        }
    }
}

/// Invalid action-selection configuration.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ActionSelectionError {
    InvalidTemperature(f32),
}

impl Display for ActionSelectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTemperature(temperature) => write!(
                formatter,
                "rank Softmax temperature must be finite and greater than zero, got {temperature}"
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

    pub(crate) fn select_index(&mut self, candidate_count: usize) -> usize {
        debug_assert!(candidate_count > 0, "validated candidate list must be nonempty");
        debug_assert!(
            candidate_count <= usize::from(self.top_k().get()),
            "validated candidate list must not exceed selector top_k"
        );
        let ActionSelectionPolicy::RankSoftmax(policy) = self.policy else {
            return 0;
        };
        if candidate_count == 1 {
            return 0;
        }

        let decay = (-1.0 / f64::from(policy.temperature())).exp();
        let total_weight = rank_weights(candidate_count, decay).sum::<f64>();
        let mut target = self.rng.random::<f64>() * total_weight;
        for (rank, weight) in rank_weights(candidate_count, decay).enumerate() {
            if target < weight {
                return rank;
            }
            target -= weight;
        }
        candidate_count - 1
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
#[cfg(test)]
mod tests {
    use super::ActionSelectionPolicy;
    use super::ActionSelector;

    #[test]
    fn invalid_temperatures_are_rejected() {
        let top_k = std::num::NonZeroU8::new(4).expect("nonzero top_k");
        for temperature in [0.0, -1.0, f32::INFINITY, f32::NAN] {
            ActionSelectionPolicy::rank_softmax(top_k, temperature)
                .expect_err("invalid temperature must be rejected");
        }
    }

    #[test]
    fn equal_seeds_reproduce_rank_samples() {
        let policy = ActionSelectionPolicy::standard_rank_softmax();
        let mut first = ActionSelector::with_seed(policy, 20260802);
        let mut second = ActionSelector::with_seed(policy, 20260802);

        let first_ranks = (0 .. 128).map(|_| first.select_index(4)).collect::<Vec<_>>();
        let second_ranks = (0 .. 128).map(|_| second.select_index(4)).collect::<Vec<_>>();

        assert_eq!(first_ranks, second_ranks);
    }

    #[test]
    fn standard_policy_favors_better_ranks() {
        let mut selector =
            ActionSelector::with_seed(ActionSelectionPolicy::standard_rank_softmax(), 20260802);
        let mut counts = [0_u32; 4];
        for _ in 0 .. 10_000 {
            counts[selector.select_index(4)] += 1;
        }

        assert!(counts[0] > counts[1]);
        assert!(counts[1] > counts[2]);
        assert!(counts[2] > counts[3]);
        assert!(counts[0] > 8_000);
    }
}
