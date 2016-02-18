use std::cell::RefCell;
use std::ffi::CStr;
use std::ffi::CString;
use std::mem;
use std::ptr;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use libc::{c_char, c_void, c_int, free};

use bindings::avahi::*;
use service_discovery::service_discovery_manager::ServiceDescription;
use service_discovery::service_discovery_manager::DiscoveryListener;
use service_discovery::service_discovery_manager::ResolveListener;

fn parse_c_string(c_string: *const c_char) -> Option<String> {
    if c_string.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(c_string) }.to_string_lossy().into_owned())
    }
}

fn parse_address(address: *const AvahiAddress) -> Option<String> {
    if address.is_null() {
        None
    } else {
        let address_vector = Vec::with_capacity(AVAHI_ADDRESS_STR_MAX).as_ptr();
        unsafe { avahi_address_snprint(address_vector, AVAHI_ADDRESS_STR_MAX, address) };

        parse_c_string(address_vector)
    }
}

fn parse_txt(txt: *mut AvahiStringList) -> Option<String> {
    if txt.is_null() {
        None
    } else {
        unsafe {
            let txt_pointer = avahi_string_list_to_string(txt);
            let txt = parse_c_string(txt_pointer);
            avahi_free(txt_pointer as *mut c_void);

            txt
        }
    }
}

#[derive(Debug)]
struct BrowseCallbackParameters {
    event: AvahiBrowserEvent,
    interface: i32,
    protocol: i32,
    name: Option<String>,
    service_type: Option<String>,
    domain: Option<String>,
    flags: AvahiLookupResultFlags,
}

#[derive(Debug)]
struct ResolveCallbackParameters {
    event: AvahiResolverEvent,
    address: Option<String>,
    interface: i32,
    port: u16,
    protocol: i32,
    name: Option<String>,
    service_type: Option<String>,
    domain: Option<String>,
    host_name: Option<String>,
    txt: Option<String>,
    flags: AvahiLookupResultFlags,
}

#[allow(unused_variables)]
extern "C" fn client_callback(s: *mut AvahiClient,
                              state: AvahiClientState,
                              userdata: *mut c_void) {
    println!("Client state changed: {:?}", state);
}

#[allow(unused_variables)]
extern "C" fn browse_callback(service_browser: *mut AvahiServiceBrowser,
                              interface: c_int,
                              protocol: c_int,
                              event: AvahiBrowserEvent,
                              name: *const c_char,
                              service_type: *const c_char,
                              domain: *const c_char,
                              flags: AvahiLookupResultFlags,
                              userdata: *mut c_void) {

    let sender = unsafe {
        mem::transmute::<*mut c_void, &Sender<BrowseCallbackParameters>>(userdata)
    };

    let parameters = BrowseCallbackParameters {
        event: event,
        interface: interface,
        protocol: protocol,
        name: parse_c_string(name),
        service_type: parse_c_string(service_type),
        domain: parse_c_string(domain),
        flags: flags,
    };

    sender.send(parameters).unwrap();
}

#[allow(unused_variables)]
extern "C" fn resolve_callback(r: *mut AvahiServiceResolver,
                               interface: c_int,
                               protocol: c_int,
                               event: AvahiResolverEvent,
                               name: *const c_char,
                               service_type: *const c_char,
                               domain: *const c_char,
                               host_name: *const c_char,
                               address: *const AvahiAddress,
                               port: u16,
                               txt: *mut AvahiStringList,
                               flags: AvahiLookupResultFlags,
                               userdata: *mut c_void) {

    let sender = unsafe {
        mem::transmute::<*mut c_void, &Sender<ResolveCallbackParameters>>(userdata)
    };

    let parameters = ResolveCallbackParameters {
        event: event,
        address: parse_address(address),
        interface: interface,
        protocol: protocol,
        port: port,
        host_name: parse_c_string(host_name),
        name: parse_c_string(name),
        service_type: parse_c_string(service_type),
        domain: parse_c_string(domain),
        txt: parse_txt(txt),
        flags: flags,
    };

    sender.send(parameters).unwrap();
}

pub struct AvahiWrapper {
    client: RefCell<Option<*mut AvahiClient>>,
    poll: RefCell<Option<*mut AvahiThreadedPoll>>,
    service_browser: RefCell<Option<*mut AvahiServiceBrowser>>,
}

impl AvahiWrapper {
    pub fn new() -> AvahiWrapper {
        AvahiWrapper {
            client: RefCell::new(None),
            poll: RefCell::new(None),
            service_browser: RefCell::new(None),
        }
    }

    pub fn start_browser<T: DiscoveryListener>(&self, service_type: &str, listener: T) {
        self.initialize_poll();
        self.initialize_client();

        let (tx, rx) = channel::<BrowseCallbackParameters>();

        let userdata = unsafe {
            mem::transmute::<&Sender<BrowseCallbackParameters>, *mut c_void>(&tx)
        };

        let avahi_service_browser = unsafe {
            avahi_service_browser_new(self.client.borrow().unwrap(),
                                      AvahiIfIndex::AVAHI_IF_UNSPEC,
                                      AvahiProtocol::AVAHI_PROTO_UNSPEC,
                                      CString::new(service_type).unwrap().as_ptr(),
                                      ptr::null_mut(),
                                      AvahiLookupFlags::AVAHI_LOOKUP_UNSPEC,
                                      *Box::new(browse_callback),
                                      userdata)
        };

        *self.service_browser.borrow_mut() = Some(avahi_service_browser);

        self.start_polling();

        for a in rx.iter() {
            match a.event {
                AvahiBrowserEvent::AVAHI_BROWSER_NEW => {
                    let service = ServiceDescription {
                        address: &"",
                        domain: &a.domain.unwrap(),
                        host_name: &"",
                        interface: a.interface,
                        name: &a.name.unwrap(),
                        port: 0,
                        protocol: a.protocol,
                        txt: &"",
                        type_name: service_type,
                    };

                    listener.on_service_discovered(service);

                    // println!("Service: {:?}", service);

                    // self.resolve(service);
                }
                AvahiBrowserEvent::AVAHI_BROWSER_ALL_FOR_NOW => {
                    listener.on_all_discovered();
                    break;
                }
                _ => println!("Default {:?}", a.event),
            }
        }
    }

    pub fn resolve<T: ResolveListener>(&self, service: ServiceDescription, listener: T) {
        let (tx, rx) = channel::<ResolveCallbackParameters>();

        let userdata = unsafe {
            mem::transmute::<&Sender<ResolveCallbackParameters>, *mut c_void>(&tx)
        };

        let avahi_service_resolver = unsafe {
            avahi_service_resolver_new(self.client.borrow().unwrap(),
                                       service.interface,
                                       service.protocol,
                                       CString::new(service.name).unwrap().as_ptr(),
                                       CString::new(service.type_name).unwrap().as_ptr(),
                                       CString::new(service.domain).unwrap().as_ptr(),
                                       AvahiProtocol::AVAHI_PROTO_UNSPEC,
                                       AvahiLookupFlags::AVAHI_LOOKUP_UNSPEC,
                                       *Box::new(resolve_callback),
                                       userdata)
        };

        // *self.service_resolver.borrow_mut() = Some(avahi_service_resolver);

        let raw_service = rx.recv().unwrap();

        let service = ServiceDescription {
            address: &raw_service.address.unwrap(),
            domain: &raw_service.domain.unwrap(),
            host_name: &raw_service.host_name.unwrap(),
            interface: raw_service.interface,
            name: &raw_service.name.unwrap(),
            port: raw_service.port,
            protocol: raw_service.protocol,
            txt: &raw_service.txt.unwrap(),
            type_name: &raw_service.service_type.unwrap(),
        };

        listener.on_service_resolved(service);

        // println!("Resolved {:?}", rx.recv().unwrap());

        unsafe {
            avahi_service_resolver_free(avahi_service_resolver);
        }
    }

    /// Creates `AvahiClient` instance for the provided `AvahiPoll` object.
    ///
    /// # Arguments
    ///
    /// * `poll` - Abstracted `AvahiPoll` object that we'd like to create client for.
    ///
    /// # Return value
    ///
    /// Initialized `AvahiClient` instance, if there was an error while creating
    /// client, this method will `panic!`.
    fn initialize_client(&self) {
        let mut client_error_code: i32 = 0;
        let poll = self.poll.borrow().unwrap();

        let avahi_client = unsafe {
            avahi_client_new(avahi_threaded_poll_get(poll),
                             AvahiClientFlags::AVAHI_CLIENT_IGNORE_USER_CONFIG,
                             *Box::new(client_callback),
                             ptr::null_mut(),
                             &mut client_error_code)
        };

        // Check that we've created client successfully, otherwise try to resolve error
        // into human-readable string.
        if avahi_client.is_null() {
            let error_string = unsafe {
                free(avahi_client as *mut c_void);
                CStr::from_ptr(avahi_strerror(client_error_code))
            };

            panic!("Failed to create avahi client: {}",
                   error_string.to_str().unwrap());
        }

        *self.client.borrow_mut() = Some(avahi_client);
    }

    fn initialize_poll(&self) {
        let avahi_poll = unsafe { avahi_threaded_poll_new() };

        *self.poll.borrow_mut() = Some(avahi_poll);
    }

    fn start_polling(&self) {
        let poll = self.poll.borrow().unwrap();

        let result_code = unsafe { avahi_threaded_poll_start(poll) };
        if result_code == -1 {
            panic!("Avahi threaded poll could not be started!");
        }
    }

    fn on_service_discovered(&self, parameters: BrowseCallbackParameters) {}
}