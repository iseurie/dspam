#[macro_use] extern crate clap;
extern crate time;
extern crate discord;

use discord::{Discord, State};
use discord::model::{Channel, ChannelId};
use std::time::Duration;
use std::io::{self, Read, Write};
use std::thread;
use clap::{Arg, App};

fn main() {
    let arg_matches = App::new("dspam")
        .version(crate_version!())
        .author(crate_authors!("\n"))
        .arg(Arg::with_name("uname")
            .required(true)
            .takes_value(true)
            .short("u")
            .value_name("USERNAME")
            .help("discord username to authenticate")
        ).arg(Arg::with_name("pass")
            .required(true)
            .takes_value(true)
            .short("p")
            .value_name("PASSWORD")
            .help("discord password to authenticate")
        ).arg(Arg::with_name("target")
            .required(true)
            .takes_value(true)
            .short("t")
            .value_name("USERNAME[#DISCRIMINATOR]")
            .help("target user identifier; takes first username match if discriminator unspecified")
        ).arg(Arg::with_name("interval")
            .takes_value(true)
            .short("i")
            .value_name("MS")
            .help("sets the delay between messages in milliseconds")
        ).arg(Arg::with_name("message")
            .takes_value(true)
            .short("m")
            .value_name("STRING")
            .help("sets the message to spam; reads from stdin if unspecified")
        ).get_matches();

    let client = Discord::new(
        arg_matches.value_of("uname").unwrap(),
        arg_matches.value_of("pass").unwrap()
    ).expect("authentication failed");
    
    let mut buf_message: String;
    let message = match arg_matches.value_of("message") {
        Some(m) => m,
        None => {
            buf_message = String::new();
            io::stdin().read_to_string(&mut buf_message).unwrap();
            &buf_message
        },
    };

    let interval_ms: u64 = if let Some(str_ms) = arg_matches.value_of("interval") {
        str_ms.parse().unwrap()
    } else {
        2000
    };

    let interval = Duration::from_millis(interval_ms);

    let target_ident = arg_matches.value_of("target").unwrap().to_string();
    let mut target_split = target_ident.split("#");
    let target = (target_split.next().unwrap(),
        if let Some(str_disc) = target_split.next() {
            str_disc.parse().ok()
        } else { None }
    );

    let target_chid = fetch_target_chid(&client, &target)
        .expect(&format!("no match for `{}' found", target_ident));
    lo_spam(&client, message, target_chid, interval);
}

fn lo_spam(client: &Discord, message: &str,
        target: ChannelId, interval: Duration) {
    loop {
        thread::sleep(interval);
        let res = client.send_message(target, message, "", false);
        if let Err(e) = res {
            let ostream = io::stderr();
            let mut writer = ostream.lock();
            let t = time::now();
            writer.write_fmt(
                format_args!(
                    "[{}:{}:{}+{}e-9] failed to send message: {}\n", 
                    t.tm_hour, t.tm_min, t.tm_sec, t.tm_nsec, e)
            ).unwrap();
        }
    }
}

fn fetch_target_chid(client: &Discord, target: &(&str, Option<u16>))
        -> Option<ChannelId> {
    let (/* connection */ _, evt_ready) = client.connect().expect("connection failed");
    let state = State::new(evt_ready);
    let dmids = state.all_private_channels();

    let check = |dmid| {
        let chan = client.get_channel(dmid).expect("failed to query DM channel");
        if let Channel::Private(dm) = chan {
            if target.0 == dm.recipient.name {
                if let Some(disc) = target.1 {
                    if disc == dm.recipient.discriminator {
                        return Some(dm.id)
                    }
                }
            }
        }
        None
    }; 

    let check_range = |range| {
        for i in range {
            let opt_target = check(dmids[i]);
            if !opt_target.is_none() { return opt_target }
        }
        None
    };

    let div = dmids.len() / 2 - 1;

    check_range(div..0);
    check_range(div+1..dmids.len());

    None
}
