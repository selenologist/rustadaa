// Implementation as per
// Bilbao, S., Esqueda Flores, F., Parker, J. D., & Välimäki, V. (2017). Antiderivative Antialiasing
// for Memoryless Nonlinearities. IEEE Signal Processing Letters, 24(7), 1049-1053.
// https://doi.org/10.1109/LSP.2017.2675541

const TOLERANCE: f64 = 1.0e-5;

pub trait NonlinearFunction {
    fn f(x: f64) -> f64;
    fn ad1(x: f64) -> f64;
    fn ad2(x: f64) -> f64;
}

pub struct HardClip {}

impl NonlinearFunction for HardClip {
    fn f(x: f64) -> f64 {
        x.clamp(-1.0, 1.0)
    }
    fn ad1(x: f64) -> f64 {
        if x.abs() < 1.0 { // if in range
            x * x / 2.0
        }
        else {
            x * x.signum() - 0.5
        }
    }
    fn ad2(x: f64) -> f64 {
        if x.abs() < 1.0 {
            x * x * x / 6.0
        }
        else {
            ((x * x / 2.0) + (1.0 / 6.0)) * x.signum() - (x / 2.0)
        }
    }
}

pub struct Tanh {}

impl NonlinearFunction for Tanh {
    fn f(x: f64) -> f64 {
        x.tanh()
    }
    fn ad1(x: f64) -> f64 {
        x.cosh().ln()
    }
    fn ad2(x: f64) -> f64 {
        use polylog::Li2; // this is probably expensive?
        let expval = (-2.0 * x).exp() + 1.0;
        0.5 * ((1.0 - expval).li2() - x * (x + 2.0 * expval.ln() - 2.0 * x.cosh().ln()))
            + (std::f64::consts::PI.powi(2) / 24.0)
    }
}

#[derive(Default)]
pub struct Adaa1 {
    x1: f64,
    ad1_x1: f64
}

impl Adaa1 {
    pub fn process<NL: NonlinearFunction>(&mut self, x: f64) -> f64 {
        let ad1_x = NL::ad1(x);

        let y =
            if (x - self.x1).abs() < TOLERANCE {
                NL::f(0.5 * (x + self.x1))
            }
            else {
                (ad1_x - self.ad1_x1) / (x - self.x1)
            };

        self.ad1_x1 = ad1_x;
        self.x1 = x;

        y
    }
}

pub struct Adaa2 {
    x_now: f64,
    x_past: f64,
    ad2_now: f64,
    d_past: f64
}

impl Default for Adaa2 {
    fn default() -> Self {
        Self {
            x_now: 0.0f64,
            x_past: 0.0f64,
            ad2_now: 0.0f64,
            d_past: 0.0f64,
        }
    }
}

impl Adaa2 {
    pub fn process<NL: NonlinearFunction>(&mut self, x_future: f64) -> f64 {
        // To calculate 2nd order ADAA we need a sample in the past and a sample in the 'future'.
        // We achieve this by basically delaying the input one sample.
        //
        // i.e. what the paper calls x^n will be called 'now'.
        // the previous sample (x^n-1) is called past.
        // the next     sample (x^n+1) is called future.

        let ad2_future = NL::ad2(x_future);

        let d_now =
            if (x_future - self.x_now).abs() <= TOLERANCE {
                // step too small, use approximation
                NL::ad1(0.5 * (x_future + self.x_now))
            }
            else {
                (ad2_future - self.ad2_now) / (x_future - self.x_now)
            };

        let y =
            if (x_future - self.x_past).abs() <= TOLERANCE {
                // step too small, use approximation
                let xbar = 0.5 * (x_future + self.x_past);
                let delta = xbar - self.x_now;
                if delta.abs() <= TOLERANCE {
                    // also too small, approximate this too
                    NL::f(0.5 * (xbar + self.x_now))
                }
                else {
                    (2.0 / delta) * (NL::ad1(xbar) + (self.ad2_now - NL::ad2(xbar)) / delta)
                }
            }
            else {
                (2.0 / (x_future - self.x_past)) * (d_now - self.d_past)
            };

        self.d_past  = d_now;
        self.x_past  = self.x_now;
        self.x_now   = x_future;
        self.ad2_now = ad2_future;

        y 
    }
}

