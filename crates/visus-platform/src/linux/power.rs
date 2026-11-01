use visus_core::Event;
use zbus::blocking::{Connection, MessageIterator, Proxy};
use zbus::message::Type;
use zbus::MatchRule;

use crate::Emit;

const SERVICE: &str = "org.freedesktop.UPower";
const DISPLAY_DEVICE: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const DEVICE: &str = "org.freedesktop.UPower.Device";

/// (on charger, percent); None on machines without a battery.
fn read(conn: &Connection) -> Option<(bool, u8)> {
    let p = Proxy::new(conn, SERVICE, DISPLAY_DEVICE, DEVICE).ok()?;
    if !p.get_property::<bool>("IsPresent").ok()? {
        return None;
    }
    let state: u32 = p.get_property("State").ok()?;
    let pct: f64 = p.get_property("Percentage").ok()?;
    // 1 charging, 4 full, 5 pending charge; 2 discharging, 3 empty, 6 pending discharge.
    let on_ac = match state {
        1 | 4 | 5 => true,
        2 | 3 | 6 => false,
        _ => return None,
    };
    Some((on_ac, pct.round().clamp(0.0, 100.0) as u8))
}

pub fn start(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-power".into())
        .spawn(move || {
            let Ok(conn) = Connection::system() else { return };
            let publish = |conn: &Connection| {
                if let Some((on_ac, percent)) = read(conn) {
                    emit(Event::Power { on_ac, percent });
                }
            };
            publish(&conn);

            let rule = MatchRule::builder()
                .msg_type(Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .and_then(|b| b.path(DISPLAY_DEVICE))
                .map(|b| b.build());
            let Ok(rule) = rule else { return };
            let Ok(iter) = MessageIterator::for_match_rule(rule, &conn, None) else { return };
            for _ in iter {
                publish(&conn);
            }
        })
        .expect("spawn power thread");
}
