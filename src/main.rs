// Copyright 2020 Parity Technologies (UK) Ltd.
// Copyright 2021 Protocol Labs.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

use clap::Parser;
use futures::stream::StreamExt;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::{identity, PeerId, Transport};
use libp2p::identify as identify;
use libp2p::ping as ping;
use libp2p::quic::Config;
use libp2p::relay as relay;
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let opt = Opt::parse();
    println!("opt: {opt:?}");

    // Create a static known PeerId based on given secret
    let local_key: identity::Keypair = generate_ed25519(opt.secret_key_seed);
    let local_peer_id = PeerId::from(local_key.public());
    println!("Local peer id: {local_peer_id:?}");

    // let tcp_transport = tcp::async_io::Transport::default();

    let transport = libp2p::quic::tokio::Transport::new(Config::new(&local_key)).map(|(p, c),_| (p, StreamMuxerBox::new(c)))
        .boxed();
    let mut relay_cfg = relay::v2::relay::Config::default();
    relay_cfg.max_circuit_bytes = 10000000000;
    relay_cfg.max_circuit_duration = std::time::Duration::from_secs(24 * 3600);
    relay_cfg.max_circuits_per_peer = 10000000;
    relay_cfg.max_circuits = 1000000;
    relay_cfg.circuit_src_rate_limiters = vec![];
    relay_cfg.reservation_rate_limiters = vec![];
    let behaviour = Behaviour {
        relay: relay::v2::relay::Relay::new(local_peer_id, relay_cfg),
        ping: ping::Behaviour::new(ping::Config::new()),
        identify: identify::Behaviour::new(identify::Config::new(
            "/TODO/0.0.1".to_string(),
            local_key.public(),
        )),
    };

    let mut swarm = Swarm::with_tokio_executor(transport, behaviour, local_peer_id);

    // Listen on all interfaces
    let listen_addr = "/ip4/0.0.0.0/udp/8890/quic-v1".parse().unwrap();
    swarm.listen_on(listen_addr)?;

    // block_on(async {
        loop {
            match swarm.next().await.expect("Infinite Stream.") {
                SwarmEvent::Behaviour(event) => {
                    println!("{event:?}")
                }
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {address:?}");
                }
                _ => {}
            }
        }
    // })
}

#[derive(NetworkBehaviour)]
#[behaviour(prelude = "libp2p::swarm::derive_prelude")]
struct Behaviour {
    relay: relay::v2::relay::Relay,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
}

fn generate_ed25519(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    let secret_key = identity::ed25519::SecretKey::from_bytes(&mut bytes)
        .expect("this returns `Err` only if the length is wrong; the length is correct; qed");
    identity::Keypair::Ed25519(secret_key.into())
}

#[derive(Debug, Parser)]
#[clap(name = "libp2p relay")]
struct Opt {
    /// Determine if the relay listen on ipv6 or ipv4 loopback address. the default is ipv4
    #[clap(long)]
    use_ipv6: Option<bool>,

    /// Fixed value to generate deterministic peer id
    #[clap(long)]
    secret_key_seed: u8,

    /// The port used to listen on all interfaces
    #[clap(long)]
    port: u16,
}
