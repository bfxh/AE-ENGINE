use crate::WastelandWorld;
use godot::prelude::*;

struct WeatherOverride {
    temperature: f32,
    humidity: f32,
    pressure: f32,
    wind_speed: f32,
    precipitation: f32,
    cloud_cover: f32,
}

#[derive(GodotClass)]
#[class(base=Node)]
struct WastelandWeather {
    world_ref: Option<Gd<WastelandWorld>>,

    #[var]
    temperature: f32,

    #[var]
    humidity: f32,

    #[var]
    pressure: f32,

    #[var]
    wind_speed: f32,

    #[var]
    precipitation: f32,

    #[var]
    cloud_cover: f32,

    #[var]
    visibility: f32,

    weather_override: Option<WeatherOverride>,
    wind_direction_x: f32,
    wind_direction_z: f32,
    storm_distance: f32,

    #[base]
    base: Base<Node>,
}

#[godot_api]
impl INode for WastelandWeather {
    fn init(base: Base<Node>) -> Self {
        Self {
            world_ref: None,
            temperature: 293.0,
            humidity: 0.5,
            pressure: 1013.25,
            wind_speed: 0.0,
            precipitation: 0.0,
            cloud_cover: 0.0,
            visibility: 10000.0,
            weather_override: None,
            wind_direction_x: 1.0,
            wind_direction_z: 0.0,
            storm_distance: f32::MAX,
            base,
        }
    }

    fn ready(&mut self) {
        if let Some(parent) = self.base().get_parent() {
            if let Ok(world) = parent.try_cast::<WastelandWorld>() {
                self.world_ref = Some(world);
            }
        }
    }

    fn process(&mut self, _delta: f64) {
        self.sync_from_world();
    }
}

#[godot_api]
impl WastelandWeather {
    fn sync_from_world(&mut self) {
        if let Some(ref world) = self.world_ref {
            let data = world.bind().export_weather_data();
            if let Some(v) = data.get("temperature") {
                self.temperature = v.to::<f32>();
            }
            if let Some(v) = data.get("humidity") {
                self.humidity = v.to::<f32>();
            }
            if let Some(v) = data.get("pressure") {
                self.pressure = v.to::<f32>();
            }
            if let Some(v) = data.get("wind_speed") {
                self.wind_speed = v.to::<f32>();
            }
            if let Some(v) = data.get("precipitation") {
                self.precipitation = v.to::<f32>();
            }
            if let Some(v) = data.get("cloud_cover") {
                self.cloud_cover = v.to::<f32>();
            }
            if let Some(v) = data.get("visibility") {
                self.visibility = v.to::<f32>();
            }
            if let Some(v) = data.get("wind_x") {
                self.wind_direction_x = v.to::<f32>();
            }
            if let Some(v) = data.get("wind_z") {
                self.wind_direction_z = v.to::<f32>();
            }
            if let Some(v) = data.get("storm_intensity") {
                let si = v.to::<f32>();
                if si > 0.01 {
                    self.storm_distance = 100.0 / si;
                } else {
                    self.storm_distance = f32::MAX;
                }
            }
        }
    }

    #[func]
    fn get_weather_at(&self, x: f32, y: f32, z: f32) -> Dictionary<Variant, Variant> {
        let alt_factor = (1.0 - y * 0.0001).max(0.3);
        let local_temp = self.temperature * alt_factor;
        let local_pressure = self.pressure * alt_factor;
        let local_humidity = self.humidity * (1.0 + y * 0.00005).min(1.0);
        let local_precip = self.precipitation * (1.0 + y * 0.00002).min(1.0);
        dict! {
            "temperature" => local_temp,
            "humidity" => local_humidity,
            "pressure" => local_pressure,
            "wind_speed" => self.wind_speed,
            "precipitation" => local_precip,
            "cloud_cover" => self.cloud_cover,
            "visibility" => self.visibility,
            "position_x" => x,
            "position_y" => y,
            "position_z" => z,
        }
    }

    #[func]
    fn get_temperature_at(&self, _x: f32, y: f32, _z: f32) -> f32 {
        let alt_factor = (1.0 - y * 0.0001).max(0.3);
        self.temperature * alt_factor
    }

    #[func]
    fn get_precipitation_at(&self, _x: f32, y: f32, _z: f32) -> f32 {
        let alt_factor = (1.0 + y * 0.00002).min(1.0);
        self.precipitation * alt_factor
    }

    fn effective_temperature(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.temperature,
            None => self.temperature,
        }
    }

    fn effective_humidity(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.humidity,
            None => self.humidity,
        }
    }

    fn effective_pressure(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.pressure,
            None => self.pressure,
        }
    }

    fn effective_wind_speed(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.wind_speed,
            None => self.wind_speed,
        }
    }

    fn effective_precipitation(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.precipitation,
            None => self.precipitation,
        }
    }

    fn effective_cloud_cover(&self) -> f32 {
        match &self.weather_override {
            Some(o) => o.cloud_cover,
            None => self.cloud_cover,
        }
    }

    #[func]
    fn set_weather_override(
        &mut self,
        temperature: f32,
        humidity: f32,
        pressure: f32,
        wind_speed: f32,
        precipitation: f32,
        cloud_cover: f32,
    ) {
        self.weather_override = Some(WeatherOverride {
            temperature,
            humidity,
            pressure,
            wind_speed,
            precipitation,
            cloud_cover,
        });
    }

    #[func]
    fn clear_weather_override(&mut self) {
        self.weather_override = None;
    }

    #[func]
    fn get_wind_direction(&self) -> Vector3 {
        let norm = (self.wind_direction_x * self.wind_direction_x
            + self.wind_direction_z * self.wind_direction_z)
            .sqrt()
            .max(0.001);
        Vector3::new(self.wind_direction_x / norm, 0.0, self.wind_direction_z / norm)
    }

    #[func]
    fn get_beaufort_scale(&self) -> i64 {
        let ws = self.effective_wind_speed();
        if ws < 0.3 {
            return 0;
        }
        if ws < 1.6 {
            return 1;
        }
        if ws < 3.4 {
            return 2;
        }
        if ws < 5.5 {
            return 3;
        }
        if ws < 8.0 {
            return 4;
        }
        if ws < 10.8 {
            return 5;
        }
        if ws < 13.9 {
            return 6;
        }
        if ws < 17.2 {
            return 7;
        }
        if ws < 20.8 {
            return 8;
        }
        if ws < 24.5 {
            return 9;
        }
        if ws < 28.5 {
            return 10;
        }
        if ws < 32.7 {
            return 11;
        }
        12
    }

    #[func]
    fn get_weather_forecast(&self, hours_ahead: f32) -> Dictionary<Variant, Variant> {
        let t = hours_ahead;
        let forecast_temp = self.effective_temperature() + (t * 0.1).sin() * 5.0;
        let forecast_humidity =
            (self.effective_humidity() + (t * 0.05).sin() * 0.1).clamp(0.0, 1.0);
        let forecast_pressure = self.effective_pressure() + (t * 0.08).sin() * 10.0;
        let forecast_wind = (self.effective_wind_speed() + (t * 0.12).sin() * 3.0).max(0.0);
        let forecast_precip = (self.effective_precipitation() + (t * 0.06).cos() * 2.0).max(0.0);
        let forecast_cloud =
            (self.effective_cloud_cover() + (t * 0.07).sin() * 0.2).clamp(0.0, 1.0);
        dict! {
            "hours_ahead" => hours_ahead,
            "temperature" => forecast_temp,
            "humidity" => forecast_humidity,
            "pressure" => forecast_pressure,
            "wind_speed" => forecast_wind,
            "precipitation" => forecast_precip,
            "cloud_cover" => forecast_cloud,
        }
    }

    #[func]
    fn get_storm_distance(&self) -> f32 {
        self.storm_distance
    }
}
