extern crate octobuild;
extern crate capnp;
extern crate hyper;
extern crate rustc_serialize;
extern crate rand;
extern crate tempdir;
#[macro_use]
extern crate log;

use octobuild::cluster::common::{BuilderInfo, RPC_BUILDER_LIST};
use octobuild::builder_capnp;

use hyper::{Client, Url};
use rustc_serialize::json;

use std::error::Error;
use std::io::{Read, Write};
use std::str::FromStr;
use std::net::{SocketAddr, TcpStream};

use capnp::serialize_packed;
use capnp::message;

fn main() {
    octobuild::utils::init_logger();

    let client = Client::new();
    match client.get(Url::parse("http://localhost:3000")
            .unwrap()
            .join(RPC_BUILDER_LIST)
            .unwrap())
        .send() {
        Ok(mut response) => {
            let mut payload = String::new();
            response.read_to_string(&mut payload).unwrap();

            let all_builders: Vec<BuilderInfo> = json::decode(&payload).unwrap();
            let builders = all_builders.into_iter()
                .filter(|b| b.toolchains.len() > 0)
                .collect();

            let builder = get_random_builder(&builders).unwrap();
            let toolchain = builder.toolchains.get(0).unwrap();

            info!("Builder: {}, {} ({})",
                  builder.endpoint,
                  builder.name,
                  toolchain);
            let addr = SocketAddr::from_str(&builder.endpoint).unwrap();

            // Connect to builder.
            let mut stream = TcpStream::connect(addr).unwrap();

            {
                // Send compilation request.
                let mut message = message::Builder::new_default();
                {
                    // Toolchain.
                    let mut request = message.init_root::<builder_capnp::compile_request::Builder>();
                    request.set_toolchain(toolchain);
                    // Arguments.
                    let mut args = request.borrow().init_args(1);
                    args.borrow().set(0, "-DFOO");
                }
                serialize_packed::write_message(&mut stream, &mut message);
            }
            {
                // Send source code.
                let mut message = message::Builder::new_default();
                {
                    // Toolchain.
                    let mut request = message.init_root::<builder_capnp::source_request::Builder>();
                    request.set_body(r#"
int main(int argc, char** argv) {
  return 0;
}
"#);
                }
                serialize_packed::write_message(&mut stream, &mut message);
            }

            let mut payload = String::new();
            stream.read_to_string(&mut payload).unwrap();
            info!("{}", payload);
        }
        Err(e) => {
            info!("Builder: can't send info to coordinator: {}",
                  e.description());
        }
    };
}

fn get_random_builder(builders: &Vec<BuilderInfo>) -> Option<&BuilderInfo> {
    if builders.len() > 0 {
        Some(&builders[rand::random::<usize>() % builders.len()])
    } else {
        None
    }
}
