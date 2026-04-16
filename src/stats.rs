// stats.rs - Runtime metrics for the AV intersection simulation.

pub struct Stats {
    pub total_vehicles_passed: u32,
    pub max_velocity: f32,
    pub min_velocity: f32,
    pub max_time: f32,
    pub min_time: f32,
    pub close_calls: u32,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            total_vehicles_passed: 0,
            max_velocity: f32::NEG_INFINITY,
            min_velocity: f32::INFINITY,
            max_time: f32::NEG_INFINITY,
            min_time: f32::INFINITY,
            close_calls: 0,
        }
    }

    pub fn observe_velocity(&mut self, velocity: f32) {
        // Ignore stop/invalid samples so min velocity reflects real movement.
        if !velocity.is_finite() || velocity <= 0.0 {
            return;
        }

        if velocity > self.max_velocity {
            self.max_velocity = velocity;
        }
        if velocity < self.min_velocity {
            self.min_velocity = velocity;
        }
    }

    pub fn record_passed_vehicle(&mut self, crossing_time: f32) {
        self.total_vehicles_passed += 1;
        if crossing_time > self.max_time {
            self.max_time = crossing_time;
        }
        if crossing_time < self.min_time {
            self.min_time = crossing_time;
        }
    }

    pub fn record_close_call(&mut self) {
        self.close_calls += 1;
    }

    pub fn report_lines(&self) -> Vec<String> {
        vec![
            "=== Smart Road Statistics ===".to_string(),
            format!(
                "Maximum number of vehicles passed: {}",
                self.total_vehicles_passed
            ),
            format!(
                "Maximum velocity reached: {}",
                fmt(self.max_velocity, "px/s")
            ),
            format!(
                "Minimum velocity reached: {}",
                fmt(self.min_velocity, "px/s")
            ),
            format!(
                "Maximum time taken by a vehicle: {}",
                fmt(self.max_time, "s")
            ),
            format!(
                "Minimum time taken by a vehicle: {}",
                fmt(self.min_time, "s")
            ),
            format!("Number of close calls: {}", self.close_calls),
        ]
    }

    pub fn dashboard_lines(&self) -> Vec<String> {
        vec![
            "SMART ROAD STATISTICS".to_string(),
            format!("MAX VEHICLES PASSED: {}", self.total_vehicles_passed),
            format!("MAX VELOCITY: {}", fmt(self.max_velocity, "PX/S")),
            format!("MIN VELOCITY: {}", fmt(self.min_velocity, "PX/S")),
            format!("MAX TIME: {}", fmt(self.max_time, "S")),
            format!("MIN TIME: {}", fmt(self.min_time, "S")),
            format!("CLOSE CALLS: {}", self.close_calls),
        ]
    }

    pub fn summary_title(&self) -> String {
        format!(
            "Stats | passed={} vmax={} vmin={} tmax={} tmin={} close={}",
            self.total_vehicles_passed,
            fmt(self.max_velocity, "px/s"),
            fmt(self.min_velocity, "px/s"),
            fmt(self.max_time, "s"),
            fmt(self.min_time, "s"),
            self.close_calls
        )
    }
}

fn fmt(v: f32, unit: &str) -> String {
    if v.is_finite() {
        format!("{:.2} {}", v, unit)
    } else {
        "N/A".to_string()
    }
}
