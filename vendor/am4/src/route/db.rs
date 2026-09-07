/*!
Implements an in-memory, pax demand and distance database.

A route constructed from one [Airport] to the other is associated
with an **undirected** pair of:
- (economy, business, first) class demands
- direct distance

We represent it as a flattened version of the
[strictly upper triangular matrix][StrictlyUpperTriangularMatrix].

Excluding routes with origin equal to the destination, there are
`n * (n - 1) / 2 = 7630371` possible routes, where `n = 3907` is the [AIRPORT_COUNT].
*/

use crate::airport::{db::AIRPORT_COUNT, Airport};
use crate::route::{demand::PaxDemand, Distance};
use crate::utils::ParseError;
use core::ops::Index;

#[cfg(feature = "rkyv")]
use rkyv::{self, util::AlignedVec};

pub const ROUTE_COUNT: usize = AIRPORT_COUNT * (AIRPORT_COUNT - 1) / 2;

/// A flattened version of the strictly upper triangular matrix.
///
/// For example, consider a world with 4 airports, the index into the [Vec] would be:
///
/// ```txt
///     0  1  2  3  <-- origin index
///    ___________
/// 0 | ·  0  1  2
/// 1 | 0  ·  3  4
/// 2 | 1  3  ·  5
/// 3 | 2  4  5  ·
///
/// ^
/// └-- destination index
/// ```
// TODO: using row and column for convenience for now, switch to index
#[derive(Debug)]
pub struct StrictlyUpperTriangularMatrix<const N: usize> {
    curr: usize,
    /// Row number
    i: usize,
    /// Column number
    j: usize,
}

impl<const N: usize> StrictlyUpperTriangularMatrix<N> {
    const CURR_MAX: usize = N * (N - 1) / 2;

    /// Compute the index of the flattened [Vec] representation,
    /// given the row and column number.
    ///
    /// Panics if `i >= j` (underflow).
    pub fn index((i, j): (usize, usize)) -> usize {
        i * (2 * N - i - 1) / 2 + (j - i - 1)
    }
}

impl<const N: usize> Default for StrictlyUpperTriangularMatrix<N> {
    fn default() -> Self {
        Self {
            curr: 0,
            i: 0,
            j: 0,
        }
    }
}

impl<const N: usize> Iterator for StrictlyUpperTriangularMatrix<N> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.curr >= Self::CURR_MAX {
            return None;
        }
        self.j += 1;
        if self.j == N {
            self.i += 1;
            self.j = self.i + 1;
        }
        self.curr += 1;
        Some((self.i, self.j))
    }
}

/// Panics if `oidx == didx` (underflow)
#[inline(always)]
fn get_index(oidx: usize, didx: usize) -> usize {
    let (i, j) = if oidx > didx {
        (didx, oidx)
    } else {
        (oidx, didx)
    };
    StrictlyUpperTriangularMatrix::<AIRPORT_COUNT>::index((i, j))
}

/// Computes the index without any runtime checks.
///
/// # Safety
/// - `oidx` and `didx` must both be `< AIRPORT_COUNT`
/// - `oidx != didx`
#[inline(always)]
fn get_index_unchecked(oidx: usize, didx: usize) -> usize {
    debug_assert!(oidx < AIRPORT_COUNT, "oidx out of bounds");
    debug_assert!(didx < AIRPORT_COUNT, "didx out of bounds");
    debug_assert_ne!(oidx, didx, "oidx == didx");

    let (i, j) = if oidx > didx {
        (didx, oidx)
    } else {
        (oidx, didx)
    };
    i * (2 * AIRPORT_COUNT - i - 1) / 2 + (j - i - 1)
}

#[derive(Debug)]
pub struct DemandMatrix(Vec<PaxDemand>);

impl DemandMatrix {
    #[cfg(feature = "rkyv")]
    pub fn from_bytes(buffer: &[u8]) -> Result<Self, ParseError> {
        // ensure serialised bytes can be deserialised
        let demands: Vec<PaxDemand> =
            rkyv::from_bytes::<Vec<PaxDemand>, rkyv::rancor::Error>(buffer)
                .map_err(|e| ParseError::ArchiveError(e.to_string()))?;

        if demands.len() != ROUTE_COUNT {
            return Err(ParseError::InvalidDataLength {
                expected: ROUTE_COUNT,
                actual: demands.len(),
            });
        }

        Ok(DemandMatrix(demands))
    }

    pub fn data(&self) -> &Vec<PaxDemand> {
        &self.0
    }
}

impl Index<(usize, usize)> for DemandMatrix {
    type Output = PaxDemand;

    /// Panics if `oidx == didx` (underflow)
    fn index(&self, (oidx, didx): (usize, usize)) -> &Self::Output {
        &self.0[get_index(oidx, didx)]
    }
}

impl DemandMatrix {
    /// Get demand without bounds checking.
    #[inline(always)]
    pub fn get_unchecked(&self, oidx: usize, didx: usize) -> PaxDemand {
        // SAFETY: Airport.idx values are guaranteed to be valid indices in [0, AIRPORT_COUNT)
        // by the database construction in `Airports::from_bytes` which assigns idx sequentially.
        // The `get_index_unchecked` function has debug asserts.
        unsafe { *self.0.get_unchecked(get_index_unchecked(oidx, didx)) }
    }
}

#[derive(Debug)]
pub struct DistanceMatrix(Vec<Distance>);

impl DistanceMatrix {
    /// Load the distance matrix from a rkyv serialised buffer
    #[cfg(feature = "rkyv")]
    pub fn from_bytes(buffer: &[u8]) -> Result<Self, ParseError> {
        let distances: Vec<_> = rkyv::from_bytes::<Vec<Distance>, rkyv::rancor::Error>(buffer)
            .map_err(|e| ParseError::ArchiveError(e.to_string()))?;

        if distances.len() != ROUTE_COUNT {
            return Err(ParseError::InvalidDataLength {
                expected: ROUTE_COUNT,
                actual: distances.len(),
            });
        }

        Ok(DistanceMatrix(distances))
    }

    /// Compute the distance matrix with haversine
    pub fn from_airports(aps: &[Airport]) -> Self {
        let d: Vec<_> = StrictlyUpperTriangularMatrix::<AIRPORT_COUNT>::default()
            .map(|(i, j)| Distance::haversine(&aps[i].location, &aps[j].location))
            .collect();
        debug_assert_eq!(d.len(), ROUTE_COUNT);
        Self(d)
    }

    #[cfg(feature = "rkyv")]
    pub fn to_bytes(&self) -> Result<AlignedVec, ParseError> {
        let av = rkyv::to_bytes(&self.0)
            .map_err(|e: rkyv::rancor::Error| ParseError::SerialiseError(e.to_string()))?;
        Ok(av)
    }

    pub fn data(&self) -> &Vec<Distance> {
        &self.0
    }
}

impl Index<(usize, usize)> for DistanceMatrix {
    type Output = Distance;

    /// Panics if `oidx == didx` (underflow)
    fn index(&self, (oidx, didx): (usize, usize)) -> &Self::Output {
        &self.0[get_index(oidx, didx)]
    }
}

impl DistanceMatrix {
    /// Get distance without bounds checking.
    #[inline(always)]
    pub fn get_unchecked(&self, oidx: usize, didx: usize) -> Distance {
        // SAFETY: Airport.idx values are guaranteed to be valid indices in [0, AIRPORT_COUNT)
        // by the database construction in `Airports::from_bytes` which assigns idx sequentially.
        // The `get_index_unchecked` function has debug asserts.
        unsafe { *self.0.get_unchecked(get_index_unchecked(oidx, didx)) }
    }
}
