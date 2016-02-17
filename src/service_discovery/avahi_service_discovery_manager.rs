use service_discovery::service_discovery_manager::*;
use service_discovery::avah_wrapper::*;

pub struct AvahiServiceDiscoveryManager {
    wrapper: AvahiWrapper,
}

impl ServiceDiscoveryManager for AvahiServiceDiscoveryManager {
    fn new() -> AvahiServiceDiscoveryManager {
        AvahiServiceDiscoveryManager { wrapper: AvahiWrapper::new() }
    }

    fn discover_services<F>(&self, service_type: &str, callback: F)
        where F: FnMut(ServiceDescription)
    {
        self.wrapper.start_browser(service_type, callback);
    }

    fn discover_services_sync<F>(&self, service_type: &str, callback: F)
        where F: FnMut(ServiceDescription)
    {
        self.wrapper.start_browser_sync(service_type, callback);
    }

    fn resolve_service<F>(&self, service_description: ServiceDescription, callback: F)
        where F: FnMut(ServiceDescription),
              F: 'static
    {
        self.wrapper.resolve(service_description, callback);
    }

    fn stop_service_discovery(&self) {
        self.wrapper.stop_browser();
    }
}
