use crate::util::{calc_w, overlap};
use ndarray::{Array, Array1, Array2, ArrayBase, Ix1, ViewRepr};
use ndrustfft::{nddct2, DctHandler, Normalization};
use rustdct::DctPlanner;

pub fn elec_field_cell(cell_loc: &Array1<f64>, bins_elec_field: &Array2<f64>, m: usize) -> f64 {
    let (cell_u, cell_v) = (cell_loc[0] as usize, cell_loc[1] as usize);
    let mut elec_field = 0.;

    //these bounds should include all the surrounding bins and the bin containing the
    //cell center
    let (u_start, u_end, v_start, v_end) = bounds_check(cell_u, cell_v, m);

    //is there a way to do this with better iterators or the like, make it prettier?
    for u in u_start..u_end {
        for v in v_start..v_end {
            let cell_overlap = overlap(&cell_loc, u as f64, v as f64);
            elec_field += cell_overlap * bins_elec_field[[u, v]];
        }
    }
    elec_field
}
enum Direction {
    X,
    Y,
}

enum SorC {
    Sin,
    Cos,
}

///calculate the a_u_vs from eq ( ) using an fft library
pub fn calc_coeffs(density: &Array2<f64>, m: usize) -> Array2<f64> {
    let handler: DctHandler<f64> = DctHandler::new(m).normalization(Normalization::None);

    let mut first_pass = Array2::<f64>::zeros((m, m));
    let mut coeffs = Array2::<f64>::zeros((m, m));

    //cosine transform on the rows
    nddct2(&density, &mut first_pass, &handler, 0);

    //cosine transform on the columns
    nddct2(&first_pass, &mut coeffs, &handler, 1);

    coeffs.mapv_inplace(|x| x / ((m as f64).powi(2)));

    coeffs
}

fn potential_coeff(w_u: f64, w_v: f64) -> f64 {
    if w_u == 0. && w_v == 0. {
        0.
    } else {
        1. / (w_u.powi(2) + w_v.powi(2))
    }
}

fn elec_coeff(u: usize, v: usize, m: usize, dir: Direction) -> f64 {
    let w_u = calc_w(u, m);
    let w_v = calc_w(v, m);

    let mut elec_coeff = 0.;

    if u != 0 && v != 0 {
        match dir {
            Direction::X => elec_coeff = w_u * potential_coeff(w_u, w_v),
            Direction::Y => elec_coeff = w_v * potential_coeff(w_u, w_v),
        }
    }
    elec_coeff
}

fn fft_row_or_col(
    row_col: &mut ArrayBase<ViewRepr<&mut f64>, Ix1>,
    planner: &mut DctPlanner<f64>,
    transform: SorC,
    m: usize,
) {
    let fft;

    match transform {
        SorC::Sin => fft = planner.plan_dst3(m),
        SorC::Cos => fft = planner.plan_dct3(m),
    }

    let mut buffer = row_col.to_vec();

    match transform {
        SorC::Sin => fft.process_dst3(&mut buffer),
        SorC::Cos => fft.process_dct3(&mut buffer),
    }

    let temp = Array::from_vec(buffer);
    row_col.assign(&temp);
}

pub fn elec_field_x(coeffs: &Array2<f64>, m: usize) -> Array2<f64> {
    //this will calculate the electric field in the x direction for each bin
    let mut elec_x = Array2::<f64>::zeros((m, m));

    let mut planner = DctPlanner::new();

    for u in 0..m {
        for v in 0..m {
            elec_x[[u, v]] = coeffs[[u, v]] * elec_coeff(u, v, m, Direction::X);
        }
    }

    //inverse cos transform on each row
    for mut row in elec_x.rows_mut() {
        fft_row_or_col(&mut row, &mut planner, SorC::Cos, m);
    }

    // inverse sin transform on each column
    for mut col in elec_x.columns_mut() {
        fft_row_or_col(&mut col, &mut planner, SorC::Sin, m);
    }

    elec_x
}

///this code is similar to elec_field_x, but writing it out to make it clear. equation 24, second half)
pub fn elec_field_y(coeffs: &Array2<f64>, m: usize) -> Array2<f64> {
    let mut planner = DctPlanner::new();
    let mut elec_y = Array2::<f64>::zeros((m, m));
    for u in 0..m {
        for v in 0..m {
            elec_y[[u, v]] = coeffs[[u, v]] * elec_coeff(u, v, m, Direction::Y);
        }
    }

    //inverse sin transform on each row
    for mut row in elec_y.rows_mut() {
        fft_row_or_col(&mut row, &mut planner, SorC::Sin, m);
    }

    // inverse cos transform on each column
    for mut col in elec_y.columns_mut() {
        fft_row_or_col(&mut col, &mut planner, SorC::Cos, m);
    }
    elec_y
}

// returns the electric field in a given direction for a cell. Expects an Array1 (may switch to tuple?)
// with the x and y coordinates of the cell. Has to take into account the overlap with all of the bins the cell
// is in. In eplace proper that'd include the stretching dsecribed in (page #), but al of our cells are larger
//than a bin so there's no stretching. Note we *could* use types to enforce that cell_loc is a 1d array of length
//2, we won't for now

///helper function for elec_field_cell, should we create an enum for it?
fn bounds_check(cell_u: usize, cell_v: usize, m: usize) -> (usize, usize, usize, usize) {
    let u_start;
    let u_end;
    let v_start;
    let v_end;

    if cell_u == 0 {
        u_start = 0
    } else {
        u_start = cell_u - 1
    };
    if cell_v == 0 {
        v_start = 0
    } else {
        v_start = cell_v - 1
    };
    if cell_u == m - 1 {
        u_end = m
    } else {
        u_end = cell_u + 2
    };
    if cell_v == m - 1 {
        v_end = m
    } else {
        v_end = cell_u + 2
    };
    (u_start, u_end, v_start, v_end)
}
