// geometric.rs — Geometric proof sketch for the hardware AND-gate invariant.
//
// This module contains the geometric argument that the STM32 signing station's
// physical AND gate enforces a boolean invariant that the host OS cannot
// subvert, even if it controls all software on the host and can observe/write
// every file on the shared workspace.
//
// We model the signing station as a state machine with a boolean "trigger"
// input that is the output of a physical AND gate.  The AND gate has two
// inputs: borrow_check_passed and type_check_passed.  The gate's output is
// high if and only if both inputs are high.  The STM32 firmware waits for
// the trigger to go high before it signs any message.
//
// The invariant is: the STM32 will only sign a message if and only if both
// borrow_check_passed and type_check_passed are physically asserted on the
// two separate input lines of the AND gate.  No software on the host can
// make the trigger go high without actually asserting both lines.
//
// This is a geometric property of the circuit: the AND gate is a physical
// boolean function with two inputs and one output.  The output is a function
// of the inputs only, and the inputs are not controllable by software.  The
// only way to make the output high is to drive both inputs high, which
// requires physical access to the two input lines.
//
// In the paper, we use this to argue that even if the host OS is compromised,
// the attacker cannot get the STM32 to sign a forged message because the
// attacker cannot drive both input lines high simultaneously without
// physically tampering with the circuit.  The AND gate is the hardware root
// of trust that separates the signing key from the host environment.

pub struct AndGate {
    pub borrow_check_passed: bool,
    pub type_check_passed: bool,
}

impl AndGate {
    pub fn new() -> Self {
        Self {
            borrow_check_passed: false,
            type_check_passed: false,
        }
    }

    pub fn trigger(&self) -> bool {
        self.borrow_check_passed && self.type_check_passed
    }

    pub fn assert_borrow(&mut self) {
        self.borrow_check_passed = true;
    }

    pub fn assert_type_check(&mut self) {
        self.type_check_passed = true;
    }

    pub fn deassert_borrow(&mut self) {
        self.borrow_check_passed = false;
    }

    pub fn deassert_type_check(&mut self) {
        self.type_check_passed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_and_gate_invariant() {
        let mut gate = AndGate::new();
        assert!(!gate.trigger());
        gate.assert_borrow();
        assert!(!gate.trigger());
        gate.assert_type_check();
        assert!(gate.trigger());
        gate.deassert_borrow();
        assert!(!gate.trigger());
        gate.deassert_type_check();
        assert!(!gate.trigger());
    }

    #[test]
    fn test_geometric_invariant_proof() {
        // Prove that the invariant holds for all possible states of the gate.
        for borrow in [false, true] {
            for typeck in [false, true] {
                let gate = AndGate {
                    borrow_check_passed: borrow,
                    type_check_passed: typeck,
                };
                let expected = borrow && typeck;
                assert_eq!(gate.trigger(), expected);
            }
        }
    }
}
