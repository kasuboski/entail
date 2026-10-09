//! Native Z3 backend for Author analysis.

#[cfg(test)]
mod tests {
    use z3::{SatResult, Solver, ast::Bool};

    #[test]
    fn native_solver_reports_sat_and_unsat() {
        let proposition = Bool::new_const("smoke_proposition");

        let sat_solver = Solver::new();
        sat_solver.assert(&proposition);
        assert_eq!(sat_solver.check(), SatResult::Sat);

        let unsat_solver = Solver::new();
        unsat_solver.assert(&proposition);
        unsat_solver.assert(proposition.not());
        assert_eq!(unsat_solver.check(), SatResult::Unsat);
    }
}
