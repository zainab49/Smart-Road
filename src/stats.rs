// stats.rs — Statistics collector for the simulation

pub struct Stats {
    pub total_vehicles: u32,
    pub max_speed: f32,
    pub min_speed: f32,
    pub max_crossing_time: f32,
    pub min_crossing_time: f32,
    pub close_calls: u32,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            total_vehicles: 0,
            max_speed: f32::NEG_INFINITY,
            min_speed: f32::INFINITY,
            max_crossing_time: f32::NEG_INFINITY,
            min_crossing_time: f32::INFINITY,
            close_calls: 0,
        }
    }

    pub fn record_vehicle(&mut self, pixels_per_tick: f32, crossing_secs: f32) {
        self.total_vehicles += 1;
        if pixels_per_tick > self.max_speed { self.max_speed = pixels_per_tick; }
        if pixels_per_tick < self.min_speed { self.min_speed = pixels_per_tick; }
        if crossing_secs > self.max_crossing_time { self.max_crossing_time = crossing_secs; }
        if crossing_secs < self.min_crossing_time { self.min_crossing_time = crossing_secs; }
    }

    pub fn record_close_call(&mut self) {
        self.close_calls += 1;
    }

    pub fn report(&self) -> Vec<String> {
        let v_max = if self.max_speed.is_finite() { format!("{:.1} px/tick", self.max_speed) } else { "N/A".into() };
        let v_min = if self.min_speed.is_finite() { format!("{:.1} px/tick", self.min_speed) } else { "N/A".into() };
        let t_max = if self.max_crossing_time.is_finite() { format!("{:.2} s", self.max_crossing_time) } else { "N/A".into() };
        let t_min = if self.min_crossing_time.is_finite() { format!("{:.2} s", self.min_crossing_time) } else { "N/A".into() };

        vec![
            format!("=== Smart Road Statistics ==="),
            format!("Total vehicles:         {}", self.total_vehicles),
            format!("Fastest speed (v_max):  {}", v_max),
            format!("Slowest speed (v_min):  {}", v_min),
            format!("Longest crossing time:  {}", t_max),
            format!("Shortest crossing time: {}", t_min),
            format!("Close calls:            {}", self.close_calls),
        ]
    }
}
