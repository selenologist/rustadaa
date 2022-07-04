use rustadaa::*;

use adaa::NonlinearFunction;

fn main() {
    // I was sanity checking this versus jatin's python version, this code isn't important
    use polylog::Li2;
    for x in [-1.0, -0.5, -0.25, 0.0, 0.25, 0.5, 1.0]{
        println!("AD1 {:.05} AD2 {:.05} li2 {:.05}", adaa::Tanh::ad1(x), adaa::Tanh::ad2(x), (1.0 - x).li2());
    }
}
