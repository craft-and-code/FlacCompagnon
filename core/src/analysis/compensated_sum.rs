//! Bounded-state addition that retains low-order terms through cancellation.
//!
//! Neumaier's correction handles either operand order; see
//! <https://doi.org/10.1002/zamm.19740540106>. Callers provide finite terms
//! whose sums fit f64. This improves accuracy, not arbitrary precision.

#[derive(Clone, Copy, Default)]
pub(super) struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    /// Add one term without discarding a smaller operand's rounding residue.
    #[inline]
    pub(super) fn push(&mut self, term: f64) {
        let next = self.sum + term;
        self.correction += if self.sum.abs() >= term.abs() {
            (self.sum - next) + term
        } else {
            (term - next) + self.sum
        };
        self.sum = next;
    }

    /// Round the corrected total into one f64 for a final measurement.
    #[inline]
    pub(super) fn total(self) -> f64 {
        self.sum + self.correction
    }

    /// Subtract cumulative snapshots without first rounding their residues.
    #[inline]
    pub(super) fn difference(self, earlier: Self) -> f64 {
        // Rounding each total first can erase an entire quiet window after a
        // large pulse. Subtract matching high/low parts before combining them.
        let mut result = Self::default();
        result.push(self.sum - earlier.sum);
        result.push(self.correction);
        result.push(-earlier.correction);
        result.total()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/analysis/compensated_sum.rs"]
mod tests;
