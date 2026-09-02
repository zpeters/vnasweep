//! Minimal complex arithmetic for the OSL calibration math. No dependency on
//! `num-complex` — only the handful of ops `cal.rs` and `sweep.rs` need.

use core::ops::{Add, Div, Mul, Sub};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

#[allow(dead_code)]
impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// Argument (phase angle) in radians, in (-π, π].
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    pub fn conj(self) -> Complex {
        Complex::new(self.re, -self.im)
    }

    pub fn is_finite(self) -> bool {
        self.re.is_finite() && self.im.is_finite()
    }
}

impl Add for Complex {
    type Output = Complex;
    fn add(self, o: Complex) -> Complex {
        Complex::new(self.re + o.re, self.im + o.im)
    }
}

impl Sub for Complex {
    type Output = Complex;
    fn sub(self, o: Complex) -> Complex {
        Complex::new(self.re - o.re, self.im - o.im)
    }
}

impl Mul for Complex {
    type Output = Complex;
    fn mul(self, o: Complex) -> Complex {
        Complex::new(
            self.re * o.re - self.im * o.im,
            self.re * o.im + self.im * o.re,
        )
    }
}

impl Mul<f64> for Complex {
    type Output = Complex;
    fn mul(self, k: f64) -> Complex {
        Complex::new(self.re * k, self.im * k)
    }
}

impl Div for Complex {
    type Output = Complex;
    fn div(self, o: Complex) -> Complex {
        let d = o.re * o.re + o.im * o.im;
        Complex::new(
            (self.re * o.re + self.im * o.im) / d,
            (self.im * o.re - self.re * o.im) / d,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Complex;

    fn close(a: Complex, b: Complex) -> bool {
        (a.re - b.re).abs() < 1e-12 && (a.im - b.im).abs() < 1e-12
    }

    #[test]
    fn arithmetic() {
        let a = Complex::new(1.0, 2.0);
        let b = Complex::new(3.0, -4.0);
        assert!(close(a + b, Complex::new(4.0, -2.0)));
        assert!(close(a - b, Complex::new(-2.0, 6.0)));
        // (1+2i)(3-4i) = 3 -4i +6i -8i^2 = 3 +2i +8 = 11 + 2i
        assert!(close(a * b, Complex::new(11.0, 2.0)));
        // (1+2i)/(3-4i) = (1+2i)(3+4i) / 25 = (3 +4i +6i +8i^2)/25 = (-5 +10i)/25
        assert!(close(a / b, Complex::new(-5.0 / 25.0, 10.0 / 25.0)));
        assert!(close(a * 2.0, Complex::new(2.0, 4.0)));
    }

    #[test]
    fn abs_and_finite() {
        assert!((Complex::new(3.0, 4.0).abs() - 5.0).abs() < 1e-12);
        assert!(Complex::new(1.0, 1.0).is_finite());
        let bad = Complex::new(1.0, 0.0) / Complex::new(0.0, 0.0);
        assert!(!bad.is_finite());
    }
}
