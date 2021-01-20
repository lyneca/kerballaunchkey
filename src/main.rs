#![feature(never_type)]

extern crate kerballaunchkey;
use kerballaunchkey::space_center;
use std::sync::{Mutex,Arc};
use std::sync::atomic::{AtomicBool, Ordering};
mod launchkey;
extern crate ctrlc;
use launchkey::{Launchkey, Message, LEDPadSet};
use failure;

fn main() -> Result<!, failure::Error> {
    let mut launchkey = Launchkey::new()?;

    let running = Arc::new(AtomicBool::new(true));
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Could not set CTRL+C handler.");

    println!("pre");
    launchkey.init();
    println!("post");

    let mut leds = LEDPadSet::new();

    while running.load(Ordering::SeqCst) {
        //println!("Getting input");
        match launchkey.messages.clone().lock() {
            Ok(mut messages) => {
                if !messages.is_empty() {
                    let message = messages.remove(0);
                    if message.status == 144 {
                        println!("recv {}", message.note);
                        if (96..104).contains(&message.note) {
                            leds.pad((message.note - 96).into()).toggle();
                        }
                        if (112..120).contains(&message.note) {
                            leds.pad((message.note - 112 + 8).into()).toggle();
                        }
                    }
                }
            }
            _ => {}
        }
        leds.show(&mut launchkey)?;
    }
    launchkey.close();

    /*
    let ksp = krpc_mars::RPCClient::connect("Server", "127.0.0.1:50000")?;

    let vessel = ksp.mk_call(&space_center::get_active_vessel())?;
    println!("Active vessel: {:?}", vessel);

    let control = ksp.mk_call(&vessel.get_control())?;
    loop {
        let brakes_on = ksp.mk_call(&control.get_brakes())?;
        println!("setting brakes to {}", !brakes_on);
        ksp.mk_call(&control.set_brakes(!brakes_on))?;
    }
    */
}
