use adapters::adapter::*;
use discovery::discovery_manager::*;

pub struct FakeAdapter;

impl DiscoveryAdapter for FakeAdapter {
    fn start_discovery(&self, service_type: &str, listeners: DiscoveryListeners) {
        FakeAdapter::print_warning();

        if listeners.on_service_discovered.is_some() {
            (*listeners.on_service_discovered.unwrap())(ServiceInfo {
                address: None,
                domain: Some(format!("local")),
                host_name: None,
                interface: 1,
                name: Some(format!("fake")),
                port: 0,
                protocol: ServiceProtocol::IPv4,
                txt: None,
                type_name: Some(service_type.to_string()),
            });
        }

        if listeners.on_all_discovered.is_some() {
            (*listeners.on_all_discovered.unwrap())();
        }
    }

    fn resolve(&self, service: ServiceInfo, listeners: ResolveListeners) {
        let service = ServiceInfo {
            address: Some(format!("192.168.1.1")),
            domain: service.domain,
            host_name: Some(format!("fake.local")),
            interface: service.interface,
            name: service.name,
            port: 80,
            protocol: service.protocol,
            txt: Some(format!("\"model=Xserve\"")),
            type_name: service.type_name,
        };

        if listeners.on_service_resolved.is_some() {
            (*listeners.on_service_resolved.unwrap())(service);
        }
    }

    fn stop_discovery(&self) {}
}

impl HostAdapter for FakeAdapter {
    fn get_name(&self) -> String {
        FakeAdapter::print_warning();
        return "fake".to_owned();
    }

    fn get_name_fqdn(&self) -> String {
        return "fake.local".to_owned();
    }

    fn set_name(&self, host_name: &str) -> String {
        FakeAdapter::print_warning();
        host_name.to_owned()
    }

    fn is_valid_name(&self, host_name: &str) -> bool {
        FakeAdapter::print_warning();
        debug!("Verifying host name: {}.", host_name);
        true
    }

    fn get_alternative_name(&self, host_name: &str) -> String {
        FakeAdapter::print_warning();
        format!("{}-2", host_name)
    }

    fn add_name_alias(&self, host_name: &str) {
        warn!("Host name change request (-> {}) will be ignored.",
              host_name);
    }
}

impl Drop for FakeAdapter {
    fn drop(&mut self) {
        debug!("There is no need to do anything, just letting you know that I'm being dropped!");
    }
}

impl Adapter for FakeAdapter {
    fn new() -> FakeAdapter {
        FakeAdapter::print_warning();
        FakeAdapter
    }
}

impl FakeAdapter {
    fn print_warning() {
        println!("WARNING: Your platform is not supported by real mDNS adapter, fake adapter is \
                  used!");
    }
}
