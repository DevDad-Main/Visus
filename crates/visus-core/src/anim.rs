/// Critically-damped-ish spring; settles exactly so the renderer can stop drawing.
#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub value: f32,
    pub target: f32,
    velocity: f32,
    stiffness: f32,
    damping: f32,
}

impl Spring {
    pub fn new(value: f32) -> Self {
        Self { value, target: value, velocity: 0.0, stiffness: 320.0, damping: 26.0 }
    }

    pub fn is_settled(&self) -> bool {
        (self.target - self.value).abs() < 0.05 && self.velocity.abs() < 0.05
    }

    pub fn step(&mut self, dt: f32) {
        if self.is_settled() {
            self.value = self.target;
            self.velocity = 0.0;
            return;
        }
        // Sub-step so large frame gaps stay stable.
        let n = (dt / 0.008).ceil().max(1.0) as u32;
        let h = dt / n as f32;
        for _ in 0..n {
            let a = self.stiffness * (self.target - self.value) - self.damping * self.velocity;
            self.velocity += a * h;
            self.value += self.velocity * h;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settles_on_target() {
        let mut s = Spring::new(0.0);
        s.target = 100.0;
        for _ in 0..300 {
            s.step(1.0 / 60.0);
        }
        assert!(s.is_settled());
        assert_eq!(s.value, 100.0);
    }
}
