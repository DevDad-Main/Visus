use visus_core::Event;
use windows::Networking::Connectivity::{NetworkConnectivityLevel, NetworkInformation, NetworkStatusChangedEventHandler};

use crate::Emit;

fn online() -> bool {
    NetworkInformation::GetInternetConnectionProfile()
        .and_then(|p| p.GetNetworkConnectivityLevel())
        .is_ok_and(|l| l == NetworkConnectivityLevel::InternetAccess)
}

/// Reports connectivity once, then on every change the OS announces.
pub fn start(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-network".into())
        .spawn(move || {
            emit(Event::Network { online: online() });
            let handler = NetworkStatusChangedEventHandler::new({
                let emit = emit.clone();
                move |_| {
                    emit(Event::Network { online: online() });
                    Ok(())
                }
            });
            let _token = NetworkInformation::NetworkStatusChanged(&handler);
            loop {
                std::thread::park();
            }
        })
        .expect("spawn network thread");
}
