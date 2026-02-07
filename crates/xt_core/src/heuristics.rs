#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchTier {
    Similar = 0,
    Partial = 1,
    Exact = 2,
}

pub fn rank_candidates(query: &str, candidates: &[String]) -> Vec<String> {
    let mut scored = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        let tier = match_tier(query, candidate);
        let score = if tier == MatchTier::Similar {
            similarity_score(query, candidate)
        } else {
            0
        };
        scored.push(ScoredCandidate {
            text: candidate.clone(),
            tier,
            score,
            index,
        });
    }

    scored.sort_by(|a, b| {
        b.tier
            .cmp(&a.tier)
            .then_with(|| b.score.cmp(&a.score))
            .then_with(|| a.index.cmp(&b.index))
    });

    scored.into_iter().map(|item| item.text).collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ScoredCandidate {
    text: String,
    tier: MatchTier,
    score: usize,
    index: usize,
}

fn match_tier(query: &str, candidate: &str) -> MatchTier {
    if candidate == query {
        MatchTier::Exact
    } else if candidate.contains(query) || query.contains(candidate) {
        MatchTier::Partial
    } else {
        MatchTier::Similar
    }
}

fn similarity_score(query: &str, candidate: &str) -> usize {
    let mut q_counts = [0u16; 256];
    let mut c_counts = [0u16; 256];
    for byte in query.bytes() {
        q_counts[byte as usize] = q_counts[byte as usize].saturating_add(1);
    }
    for byte in candidate.bytes() {
        c_counts[byte as usize] = c_counts[byte as usize].saturating_add(1);
    }
    q_counts
        .iter()
        .zip(c_counts.iter())
        .map(|(a, b)| (*a).min(*b) as usize)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_heu_001_ranking_order() {
        let query = "hello".to_string();
        let candidates = vec![
            "hello".to_string(),
            "hello there".to_string(),
            "hxllo".to_string(),
            "world".to_string(),
        ];
        let ranked = rank_candidates(&query, &candidates);
        assert_eq!(ranked[0], "hello");
        assert_eq!(ranked[1], "hello there");
        assert_eq!(ranked[2], "hxllo");
    }
}
