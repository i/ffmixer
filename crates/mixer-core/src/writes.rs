//! Plans the gain writes that move the unit toward the model, safely.
//!
//! [`plan`] is called once per writer tick (every 10 ms in the app) with
//! what the unit holds and what the model wants. It returns at most
//! `limit` writes, quietest first. Decreases happen at once; increases
//! are **ramped**: a gain rises by at most [`RAMP_STEP_DB`] per tick,
//! starting from [`RAMP_FLOOR_DB`] when coming from −∞, so a route never
//! jumps to full level (−60 dB to 0 dB takes about 0.2 s).
//!
//! Phase (sign) changes are written like any other change, ramping only
//! if the magnitude rises. FFADO's driver warns that the hardware spikes
//! on sign changes; on this FF800 a test tone showed no overshoot on any
//! sign transition, including FFADO's 0 → −1 case (see
//! `docs/totalmix-fx-reference.md`).

use crate::law::DBUS_UNITY;
use crate::mixer::Crosspoints;

/// Largest gain increase per tick.
pub const RAMP_STEP_DB: f64 = 3.0;
/// Level a ramp starts from when the route was silent.
pub const RAMP_FLOOR_DB: f64 = -60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Matrix {
    Inputs,
    Playbacks,
    /// Output volumes; `row` is always 0.
    Outputs,
}

/// One gain to send: D-Bus units, `[row][col]` = `[output][source]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Write {
    pub matrix: Matrix,
    pub row: usize,
    pub col: usize,
    pub value: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// Writes for this tick, decreases first.
    pub writes: Vec<Write>,
    /// Writes beyond `limit`, left for the next tick.
    pub deferred: usize,
}

/// The value to send to one crosspoint this tick, or `None` if settled.
pub fn step(
    current: f64,
    target: f64,
) -> Option<f64> {
    if current == target {
        return None;
    }
    let sign = if target < 0.0 { -1.0 } else { 1.0 };
    let (now, wanted) = (current.abs(), target.abs());
    if wanted <= now {
        return Some(target);
    }
    let floor = DBUS_UNITY * 10f64.powf(RAMP_FLOOR_DB / 20.0);
    let ceiling = (now.max(floor) * 10f64.powf(RAMP_STEP_DB / 20.0)).round();
    Some(sign * wanted.min(ceiling.max(now + 1.0)))
}

pub fn plan(
    unit: &Crosspoints,
    target: &Crosspoints,
    limit: usize,
) -> Plan {
    let mut decreases = Vec::new();
    let mut increases = Vec::new();
    for (matrix, row, col) in crosspoints() {
        let current = get(unit, matrix, row, col);
        let Some(value) = step(current, get(target, matrix, row, col)) else {
            continue;
        };
        let write = Write {
            matrix,
            row,
            col,
            value,
        };
        if value.abs() < current.abs() {
            decreases.push(write);
        } else {
            increases.push(write);
        }
    }
    let mut writes = decreases;
    writes.append(&mut increases);
    let deferred = writes.len().saturating_sub(limit);
    writes.truncate(limit);
    Plan { writes, deferred }
}

/// Records a write the unit accepted.
pub fn apply(
    unit: &mut Crosspoints,
    write: &Write,
) {
    match write.matrix {
        Matrix::Inputs => unit.inputs[write.row][write.col] = write.value,
        Matrix::Playbacks => unit.playbacks[write.row][write.col] = write.value,
        Matrix::Outputs => unit.outputs[write.col] = write.value,
    }
}

fn get(
    points: &Crosspoints,
    matrix: Matrix,
    row: usize,
    col: usize,
) -> f64 {
    match matrix {
        Matrix::Inputs => points.inputs[row][col],
        Matrix::Playbacks => points.playbacks[row][col],
        Matrix::Outputs => points.outputs[col],
    }
}

fn crosspoints() -> impl Iterator<Item = (Matrix, usize, usize)> {
    let square = |matrix| {
        (0..crate::CHANNELS)
            .flat_map(move |row| (0..crate::CHANNELS).map(move |col| (matrix, row, col)))
    };
    square(Matrix::Inputs)
        .chain(square(Matrix::Playbacks))
        .chain((0..crate::CHANNELS).map(|col| (Matrix::Outputs, 0, col)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNITY: f64 = DBUS_UNITY;

    /// Runs ticks until settled; returns every value sent.
    fn ramp(
        from: f64,
        to: f64,
    ) -> Vec<f64> {
        let mut current = from;
        let mut sent = Vec::new();
        while let Some(value) = step(current, to) {
            sent.push(value);
            current = value;
            assert!(sent.len() < 100, "ramp never settles");
        }
        sent
    }

    #[test]
    fn decreases_are_immediate() {
        assert_eq!(step(UNITY, 100.0), Some(100.0));
        assert_eq!(step(UNITY, 0.0), Some(0.0));
    }

    #[test]
    fn increases_rise_at_most_three_db_per_tick() {
        let sent = ramp(0.0, UNITY);
        assert_eq!(*sent.last().unwrap(), UNITY);
        // -60 dB to 0 dB at 3 dB per tick.
        assert!((20..=22).contains(&sent.len()), "{}", sent.len());
        let mut previous = UNITY * 10f64.powf(RAMP_FLOOR_DB / 20.0);
        for value in sent {
            let rise = 20.0 * (value / previous).log10();
            assert!(rise <= RAMP_STEP_DB + 0.1, "{rise} dB");
            previous = value;
        }
    }

    #[test]
    fn small_moves_go_straight_through() {
        assert_eq!(step(UNITY, UNITY * 1.2), Some(UNITY * 1.2));
    }

    #[test]
    fn phase_flip_at_same_level_is_one_write() {
        assert_eq!(step(UNITY, -UNITY), Some(-UNITY));
        assert_eq!(step(-UNITY, UNITY), Some(UNITY));
    }

    #[test]
    fn inverted_routes_ramp_up_too() {
        let sent = ramp(0.0, -UNITY);
        assert!(sent.len() > 10);
        assert!(sent.iter().all(|v| *v < 0.0));
        assert_eq!(*sent.last().unwrap(), -UNITY);
    }

    #[test]
    fn plan_sends_decreases_first_and_respects_limit() {
        let unit = Crosspoints {
            outputs: vec![UNITY; crate::CHANNELS],
            ..Crosspoints::silent()
        };
        let mut target = unit.clone();
        target.inputs[0][0] = UNITY;
        target.outputs[5] = 0.0;
        target.playbacks[1][1] = UNITY;

        let plan = plan(&unit, &target, 1);
        assert_eq!(plan.writes.len(), 1);
        assert_eq!(plan.writes[0].matrix, Matrix::Outputs);
        assert_eq!(plan.writes[0].value, 0.0);
        assert_eq!(plan.deferred, 2);
    }

    #[test]
    fn plan_converges_to_target() {
        let mut unit = Crosspoints::silent();
        let mut target = Crosspoints::silent();
        target.inputs[3][4] = UNITY * 2.0;
        target.outputs[0] = UNITY;
        for _ in 0..50 {
            for write in plan(&unit, &target, 64).writes {
                apply(&mut unit, &write);
            }
        }
        assert_eq!(unit, target);
    }
}
