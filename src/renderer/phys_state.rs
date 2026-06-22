pub struct PhysState {
    physics_accumulator: f32,
    /// Time warp
    pub traversal_modifier: f32,
}

impl PhysState {
    pub fn new() -> PhysState {
        PhysState {
            physics_accumulator: 0.0,
            traversal_modifier: 1.0,
        }
    }

    // pub fn cycle<F>(&mut self, prev_frame_dt: f32, cycle_dt_target: f32, mut f: F)
    // where
    //     F: FnMut(),
    // {
    //     self.physics_accumulator += prev_frame_dt * self.traversal_modifier.abs();
    //     // Give up on keeping rtf 1 if we're just fucked anyway
    //     self.physics_accumulator = self.physics_accumulator.min(0.25);

    //     while self.physics_accumulator >= cycle_dt_target {
    //         f();

    //         self.physics_accumulator -= cycle_dt_target;
    //     }
    // }

    pub fn get_cycle_target(&mut self, prev_frame_dt: f32, cycle_dt_target: f32) -> u32 {
        self.physics_accumulator += prev_frame_dt * self.traversal_modifier.abs();
        // Give up on keeping rtf 1 if we're just fucked anyway
        self.physics_accumulator = self.physics_accumulator.min(1.0);

        let target_cycles = (self.physics_accumulator / cycle_dt_target).floor();
        self.physics_accumulator -= cycle_dt_target * target_cycles;

        target_cycles.floor() as u32
    }

    pub fn get_debt(&self) -> f32 {
        self.physics_accumulator
    }
}
