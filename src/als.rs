use faer::{linalg::solvers::Solve, sparse::SparseColMat, Col};

pub fn als(
    sparse: SparseColMat<usize, f32>,
    iter_num: usize,
    rows_per_col: usize,
) -> (Col<f32>, Col<f32>) {
    // LU decomposition:
    let lu = sparse.sp_lu().unwrap();
    let mut first = Col::from_fn(rows_per_col, |_| 1.0);
    let mut second = lu.solve(&first);
    let mut odd = true;
    for _x in 0..iter_num {
        if odd {
            first = lu.solve(&second);
        } else {
            second = lu.solve(&first);
        }
        odd = !odd;
    }
    (first, second)
}

// Likely don't need to do als, as svd will give me two columns whose product maps to this
// But can't do it with sparse matrix
//
