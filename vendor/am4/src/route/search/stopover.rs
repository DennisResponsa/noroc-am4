use crate::{
    aircraft::Aircraft,
    airport::Airport,
    route::{db::DistanceMatrix, Distance},
    user::GameMode,
};

#[derive(Debug, Clone)]
pub struct Stopover<'a>(pub &'a Airport);

impl<'a> Stopover<'a> {
    /// given an origin O, destination D, and range R, find the best intermediate stopover S,
    /// such that distance(O, S) + distance(S, D) is minimised, subject to distance < R.
    pub fn find_by_efficiency(
        airports: &'a [Airport],
        distances: &DistanceMatrix,
        origin: &Airport,
        destination: &Airport,
        aircraft: &Aircraft,
        game_mode: GameMode,
    ) -> Option<(Self, Distance)> {
        let mut best_stopover_idx: Option<usize> = None;
        let mut best_dist_total = Distance::MAX;

        let rwy_req = if game_mode == GameMode::Realism {
            aircraft.rwy
        } else {
            0
        };
        let range = aircraft.range as f32;
        let origin_idx = origin.idx;
        let dest_idx = destination.idx;

        for (idx, candidate) in airports.iter().enumerate() {
            if candidate.idx == origin_idx || candidate.idx == dest_idx {
                continue;
            }
            if candidate.rwy < rwy_req {
                continue;
            }

            let inbound_dist = distances.get_unchecked(origin_idx, candidate.idx);
            if inbound_dist < Distance::MIN || inbound_dist.get() > range {
                continue;
            }

            let outbound_dist = distances.get_unchecked(dest_idx, candidate.idx);
            if outbound_dist < Distance::MIN || outbound_dist.get() > range {
                continue;
            }

            let dist_total = inbound_dist + outbound_dist;
            if dist_total < best_dist_total {
                best_stopover_idx = Some(idx);
                best_dist_total = dist_total;
            }
        }

        best_stopover_idx.map(|idx| (Self(&airports[idx]), best_dist_total))
    }

    /// Find a stopover that inflates the total distance to be as close to `target_distance` as possible
    /// without exceeding it, for contribution maximization.
    pub fn find_by_target_distance_lt(
        airports: &'a [Airport],
        distances: &DistanceMatrix,
        origin: &Airport,
        destination: &Airport,
        aircraft: &Aircraft,
        game_mode: GameMode,
        target_distance: Distance,
    ) -> Option<(Self, Distance)> {
        let mut best_stopover_idx: Option<usize> = None;
        let mut best_dist_total = Distance::MIN;

        let rwy_req = if game_mode == GameMode::Realism {
            aircraft.rwy
        } else {
            0
        };
        let range = aircraft.range as f32;
        let origin_idx = origin.idx;
        let dest_idx = destination.idx;
        let target_val = target_distance.get();

        for (idx, candidate) in airports.iter().enumerate() {
            if candidate.idx == origin_idx || candidate.idx == dest_idx {
                continue;
            }
            if candidate.rwy < rwy_req {
                continue;
            }

            let inbound_dist = distances.get_unchecked(origin_idx, candidate.idx);
            if inbound_dist < Distance::MIN || inbound_dist.get() > range {
                continue;
            }

            let outbound_dist = distances.get_unchecked(dest_idx, candidate.idx);
            if outbound_dist < Distance::MIN || outbound_dist.get() > range {
                continue;
            }

            let dist_total = inbound_dist + outbound_dist;
            if dist_total.get() <= target_val && dist_total > best_dist_total {
                best_stopover_idx = Some(idx);
                best_dist_total = dist_total;
                // does the contribution formula operate on float or rounded values?
                // just to be safe we early exit if we're within rounding epsilon
                if (target_val - dist_total.get()).abs() < 0.5 {
                    break;
                }
            }
        }

        best_stopover_idx.map(|idx| (Self(&airports[idx]), best_dist_total))
    }
}
