pub fn quint_ease_in(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if d == 0.0 {
        return b;
    }
    let t = t / d;
    c * t * t * t * t * t + b
}

pub fn quint_ease_out(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if d == 0.0 {
        return b + c;
    }
    let t = t / d - 1.0;
    c * (t * t * t * t * t + 1.0) + b
}

pub fn linear(t: f32, b: f32, c: f32, d: f32) -> f32 {
    if d == 0.0 {
        return b + c;
    }
    c * t / d + b
}
