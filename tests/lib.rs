#[cfg(test)]
mod common;
mod control;
mod no_lattice;
mod with_lattice;

use std::env::temp_dir;

#[cfg(test)]
#[ctor::ctor]
fn init() {
    println!("Purging provider cache");
    let path = temp_dir();
    let path = path.join("wasmcloudcache");
    let _ = ::std::fs::remove_dir_all(path);
}

#[actix_rt::test]
async fn unlink_provider() {
    let res = no_lattice::unlink_provider().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn distributed_unlink_provider() {
    let res = with_lattice::distributed_unlink().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn live_update() {
    let res = control::live_update().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn multiple_ocirefs() {
    let res = control::multiple_ocirefs().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn actor_to_actor_call_alias() {
    let res = no_lattice::actor_to_actor_call_alias().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn empty_host_has_two_providers() {
    let res = no_lattice::empty_host_has_two_providers().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn start_and_stop_actor() {
    let res = no_lattice::start_and_stop_actor().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn start_and_execute_echo() {
    let res = no_lattice::start_and_execute_echo().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn kvcounter_basic() {
    let res = no_lattice::kvcounter_basic().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn kvcounter_start_stop() {
    let res = no_lattice::kvcounter_start_stop().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn kvcounter_link_first() {
    let res = no_lattice::kvcounter_link_first().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
#[ignore]
async fn distributed_echo() {
    let res = with_lattice::distributed_echo().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn link_on_third_host() {
    let res = with_lattice::link_on_third_host().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn redis_kvcache() {
    let res = with_lattice::redis_kvcache().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn extras_provider() {
    let res = no_lattice::extras_provider().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

//TODO: get this test working in a way that doesn't require specific time delays
//#[actix_rt::test]
//async fn scaled_kvcounter() {
//    let res = with_lattice::scaled_kvcounter().await;
//    if let Err(ref e) = res {
//        println!("{}", e);
//    }
//    assert!(res.is_ok());
//}

#[actix_rt::test]
async fn control_auctions() {
    let res = control::auctions().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn control_calltest() {
    let res = control::calltest().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn monitor_event_stream() {
    let res = control::monitor_event_stream().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}

#[actix_rt::test]
async fn cant_use_unstarted_host() {
    let res = no_lattice::cant_use_unstarted_host().await;
    if let Err(ref e) = res {
        println!("{}", e);
    }
    assert!(res.is_ok());
}
