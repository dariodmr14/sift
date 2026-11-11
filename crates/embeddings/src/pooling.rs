//! De «un vector por token» a «un vector por texto», y normalizado.

use crate::Pooling;

/// `hidden`: salida del modelo, `[textos × tokens × dims]` en fila. `mask`: 1 para los
/// tokens de verdad y 0 para el relleno, `[textos × tokens]`. Devuelve un vector de
/// longitud 1 por texto.
pub fn pool(
    hidden: &[f32],
    mask: &[i64],
    texts: usize,
    tokens: usize,
    dims: usize,
    pooling: Pooling,
) -> Vec<Vec<f32>> {
    (0..texts)
        .map(|t| {
            let row = |k: usize| &hidden[(t * tokens + k) * dims..(t * tokens + k + 1) * dims];
            let mut v = match pooling {
                Pooling::Cls => row(0).to_vec(),
                Pooling::Mean => {
                    let mut sum = vec![0.0f32; dims];
                    let mut n = 0.0f32;
                    for k in 0..tokens {
                        if mask[t * tokens + k] != 0 {
                            for (s, x) in sum.iter_mut().zip(row(k)) {
                                *s += x;
                            }
                            n += 1.0;
                        }
                    }
                    let n = n.max(1.0);
                    sum.iter_mut().for_each(|s| *s /= n);
                    sum
                }
            };
            normalize(&mut v);
            v
        })
        .collect()
}

/// Divide por la longitud: así el coseno es el producto escalar.
pub fn normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter_mut().for_each(|x| *x /= norm);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Dos textos, tres tokens, dos dimensiones. El segundo texto tiene relleno al final.
    const HIDDEN: [f32; 12] = [
        3.0, 4.0, 1.0, 0.0, 0.0, 1.0, // texto 0
        0.0, 2.0, 2.0, 0.0, 9.0, 9.0, // texto 1 (el último token es relleno)
    ];
    const MASK: [i64; 6] = [1, 1, 1, 1, 1, 0];

    #[test]
    fn cls_takes_the_first_token_normalized() {
        let v = pool(&HIDDEN, &MASK, 2, 3, 2, Pooling::Cls);
        assert_eq!(v[0], [0.6, 0.8]);
        assert_eq!(v[1], [0.0, 1.0]);
    }

    #[test]
    fn mean_ignores_padding() {
        let v = pool(&HIDDEN, &MASK, 2, 3, 2, Pooling::Mean);
        // Texto 1: media de (0,2) y (2,0) = (1,1); el (9,9) del relleno no cuenta.
        let s = std::f32::consts::FRAC_1_SQRT_2;
        assert!((v[1][0] - s).abs() < 1e-6 && (v[1][1] - s).abs() < 1e-6);
        let len: f32 = v[0].iter().map(|x| x * x).sum();
        assert!((len - 1.0).abs() < 1e-6);
    }
}
