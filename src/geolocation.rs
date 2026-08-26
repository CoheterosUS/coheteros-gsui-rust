use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use windows::Devices::Geolocation::Geolocator;

pub fn spawn_location_poller(pos: Arc<Mutex<Option<(f64, f64)>>>) {
    thread::spawn(move || {
        let Ok(locator) = Geolocator::new() else {
            return;
        };
        loop {
            if let Ok(result) = locator.GetGeopositionAsync() {
                if let Ok(geopos) = result.get() {
                    let coord = geopos.Coordinate().unwrap();
                    let point = coord.Point().unwrap();
                    let position = point.Position().unwrap();
                    let mut lock = pos.lock().unwrap();
                    *lock = Some((position.Latitude, position.Longitude));
                }
            }
            thread::sleep(Duration::from_secs(5));
        }
    });
}
