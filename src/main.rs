extern crate ens;
extern crate ipfsapi;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate wasmi;
extern crate web3;

use ens::ENS;
use ipfsapi::IpfsApi;
use futures::Stream;
use std::cmp;
use std::str;
use std::sync::Arc;
use tokio_core::reactor::{self, Handle};
use wasmi::{Module, ModuleInstance, ImportsBuilder, ModuleImportResolver, RuntimeValue, FuncRef, TableRef, MemoryRef, GlobalRef, Signature, GlobalDescriptor, MemoryDescriptor, TableDescriptor, FuncInstance, ValueType, RuntimeArgs, Trap, Externals, ExternVal};
use web3::Web3;
use web3::futures::Future;

use hyper::{Chunk, StatusCode};
use hyper::header::{ContentLength, Host};
use hyper::server::{Http, Service, Request, Response};

struct Echo {
    handle: Handle,
    ens: Arc<ENS<web3::transports::Http>>,
    ipfs: Arc<IpfsApi>,
}

enum Method {
    Get = 1,
    Post = 2,
    Put = 3,
    Delete = 4
}

impl Service for Echo {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=hyper::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let hash = match req.headers().get::<Host>() {
            Some(host) => {
                let hostname = host.hostname();
                let hostname = hostname.replace(".eth-gw.uk.to", ".eth"); // TODO Set up ENS so that my real domain resolves
                // Unfortunately, ENS content hashes are 32 bytes, while IPFS CIDs are 34, so
                // combine with the address. :(
                self.ens.address("eth", &hostname).join(self.ens.content("eth", &hostname))
                    //.then(move |x| { println!("ENS resolution {}: {:?}", hostname, x); x })
                    .map(|(address, content)|  {
                        let mut s: [u8; 52] = [0; 52];
                        address.copy_to(&mut s[..20]);
                        content.copy_to(&mut s[20..]);
                        let s = String::from_utf8(s[..46].to_vec()).unwrap();
                        s
                    })
                    .map_err(|_| hyper::Error::Timeout)
            },
            None => return Box::new(futures::future::ok(
                Response::new()
                    .with_status(StatusCode::NotFound))),
        };
        
        let ipfs = self.ipfs.clone();
        Box::new(hash.and_then(move |hash| {

            let bytes = ipfs.cat(&hash).unwrap();
            let bytes: Vec<u8> = bytes.collect();
        
            let module = Module::from_buffer(bytes)
                .expect("failed to load wasm");
        
            let imports = ImportsBuilder::new()
                .with_resolver("env", &EnvModuleResolver);
            let instance =
                ModuleInstance::new(
                    &module,
                    &imports
                )
                .expect("failed to instantiate wasm module")
                .assert_no_start();
        
            let memory = match instance.export_by_name("memory") {
                Some(ExternVal::Memory(memory)) => memory,
                _ => panic!("memory is not a memory!"),
            };
        
            let mut externals = HostExternals {
                memory,
                request: req,
                response: Response::new(),
            };
        
            // Finally, invoke the exported function "test" with no parameters
            // and empty external function executor.
            assert_eq!(
                instance.invoke_export(
                    "handle",
                    &[],
                    &mut externals,
                ).expect("failed to execute export"),
                None
            );
    
            futures::future::ok(externals.response)
        }))
    }

}

#[derive(Copy, Clone)]
enum FuncIndex {
    GetRequestMethod,
    GetRequestUrl,
    GetRequestUrlLen,
    GetRequestHeader,
    GetRequestHeaderLen,
    GetRequestBody,
    GetRequestBodyLen,
    SetResponseStatus,
    SetResponseHeader,
    SetResponseBody,
    Trace,
}

struct HostExternals {
    memory: MemoryRef,
    request: Request,
    response: Response,
}

impl Externals for HostExternals {
    fn invoke_index(
        &mut self,
        index: usize,
        args: RuntimeArgs,
    ) -> Result<Option<RuntimeValue>, Trap> {
        match index {
            x if x == FuncIndex::GetRequestMethod as usize => {
                let method = match self.request.method() {
                    &hyper::Get => Method::Get,
                    &hyper::Post => Method::Post,
                    &hyper::Put => Method::Put,
                    &hyper::Delete => Method::Delete,
                    _ => Method::Get, // TODO Implement the other methods
                };
                Ok(Some(RuntimeValue::I32(method as i32)))
            },
            x if x == FuncIndex::GetRequestUrl as usize => {
                let ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                let len = cmp::min(len, self.request.uri().as_ref().len() as u32);
                self.memory.set(ptr, &self.request.uri().as_ref().as_bytes()[..len as usize]).unwrap();
                Ok(Some(RuntimeValue::I32(len as i32)))
            },
            x if x == FuncIndex::GetRequestUrlLen as usize => {
                let len = self.request.uri().as_ref().len();
                Ok(Some(RuntimeValue::I32(len as i32)))
            },
            x if x == FuncIndex::GetRequestHeader as usize => {
                panic!("unimplemented")
            },
            x if x == FuncIndex::GetRequestHeaderLen as usize => {
                panic!("unimplemented")
            },
            x if x == FuncIndex::GetRequestBody as usize => {
                panic!("unimplemented")
            },
            x if x == FuncIndex::GetRequestBodyLen as usize => {
                let len = self.request.headers().get::<ContentLength>().unwrap().0;
                Ok(Some(RuntimeValue::I32(len as i32)))
            },
            x if x == FuncIndex::SetResponseStatus as usize => {
                let status: u32 = args.nth_checked(0)?;
                self.response.set_status(StatusCode::try_from(status as u16).unwrap());
                Ok(None)
            },
            x if x == FuncIndex::SetResponseHeader as usize => {
                let hdr_ptr: u32 = args.nth_checked(0)?;
                let hdr_len: u32 = args.nth_checked(1)?;
                let val_ptr: u32 = args.nth_checked(2)?;
                let val_len: u32 = args.nth_checked(3)?;
                let hdr = String::from_utf8(self.memory.get(hdr_ptr, hdr_len as usize).unwrap()).unwrap();
                let val = String::from_utf8(self.memory.get(val_ptr, val_len as usize).unwrap()).unwrap();
                self.response.headers_mut().set_raw(hdr, val);
                Ok(None)
            },
            x if x == FuncIndex::SetResponseBody as usize => {
                let ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                self.response.set_body(String::from_utf8(self.memory.get(ptr, len as usize).unwrap()).unwrap());
                Ok(None)
            },
            x if x == FuncIndex::Trace as usize => {
                let ptr: u32 = args.nth_checked(0)?;
                let len: u32 = args.nth_checked(1)?;
                println!("TRACE: {}", str::from_utf8(&self.memory.get(ptr, len as usize).unwrap()).unwrap());
                Ok(None)
            },
            _ => panic!("Unimplemented function at {}", index),
        }
    }
}

struct EnvModuleResolver;

impl ModuleImportResolver for EnvModuleResolver {
    fn resolve_func(
        &self, 
        field_name: &str, 
        _signature: &Signature
    ) -> Result<FuncRef, wasmi::Error> {
        match field_name {
            "_get_request_method" => Ok(FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestMethod as usize)),
            "_get_request_url" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestUrl as usize)),
            "_get_request_url_len" => Ok(FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestUrlLen as usize)),
            "_get_request_header" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestHeader as usize)),
            "_get_request_body" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestBody as usize)),
            "_get_request_body_len" => Ok(FuncInstance::alloc_host(
                Signature::new(&[][..], Some(ValueType::I32)),
                    FuncIndex::GetRequestBodyLen as usize)),
            "_set_response_status" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32][..], None),
                    FuncIndex::SetResponseStatus as usize)),
            "_set_response_header" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32, ValueType::I32, ValueType::I32][..], None),
                    FuncIndex::SetResponseHeader as usize)),
            "_set_response_body" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                    FuncIndex::SetResponseBody as usize)),
            "_trace" => Ok(FuncInstance::alloc_host(
                Signature::new(&[ValueType::I32, ValueType::I32][..], None),
                    FuncIndex::Trace as usize)),
            _ => Err(wasmi::Error::Instantiation(format!("Function {} doesn't exist", field_name))),
        }
    }

    fn resolve_global(
        &self, 
        field_name: &str, 
        _global_type: &GlobalDescriptor
    ) -> Result<GlobalRef, wasmi::Error> {
        Err(wasmi::Error::Instantiation(format!("Global {} doesn't exist", field_name)))
    }

    fn resolve_memory(
        &self, 
        field_name: &str, 
        _memory_type: &MemoryDescriptor
    ) -> Result<MemoryRef, wasmi::Error> {
        Err(wasmi::Error::Instantiation(format!("Memory {} doesn't exist", field_name)))
    }

    fn resolve_table(
        &self, 
        field_name: &str, 
        _table_type: &TableDescriptor
    ) -> Result<TableRef, wasmi::Error> {
        Err(wasmi::Error::Instantiation(format!("Table {} doesn't exist", field_name)))
    }
}


fn main() {
    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    let transport = web3::transports::Http::with_event_loop("http://localhost:8545", &handle, 64).unwrap();
    let web3 = Web3::new(transport);
    let ens = ENS::with_ens_addr(web3, "adb9e045ff13e72662d541eb334c59f4634ef8b0".parse().unwrap());
    let ens = Arc::new(ens);

    let ipfs = IpfsApi::new("127.0.0.1", 5001);
    let ipfs = Arc::new(ipfs);
  
    let http = Http::<Chunk>::new();
    let addr = "0.0.0.0:8888".parse().unwrap();
    let server = http.serve_addr_handle(&addr, &handle, {
        let handle = handle.clone();
        let ens = ens.clone();
        let ipfs = ipfs.clone();
        move || Ok(Echo{handle: handle.clone(), ens: ens.clone(), ipfs: ipfs.clone()})
    }).unwrap();
    handle.spawn(server.for_each({
        let handle = handle.clone();
        move |conn| {
            handle.spawn(conn.map(|_| ()).map_err(|err| println!("server error: {:?}", err)));
            Ok(())
        }
    }).map_err(|_| ()));

    core.run(futures::future::empty::<(), ()>()).unwrap();
}
