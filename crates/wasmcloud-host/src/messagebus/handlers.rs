use super::MessageBus;
use crate::dispatch::{Invocation, InvocationResponse, WasccEntity};
use crate::hlreg::HostLocalSystemService;
use crate::messagebus::rpc_client::RpcClient;
use crate::messagebus::rpc_subscription::{CreateSubscription, RpcSubscription};
use crate::messagebus::{
    AdvertiseBinding, AdvertiseClaims, CanInvoke, ClaimsResponse, FindBindings,
    FindBindingsResponse, GetClaims, Initialize, LinkDefinition, LinksResponse, LookupBinding,
    PutClaims, PutLink, QueryActors, QueryAllLinks, QueryProviders, QueryResponse, Subscribe,
    Unsubscribe,
};
use crate::{auth, Result};
use actix::prelude::*;
use std::sync::Arc;

pub const OP_PERFORM_LIVE_UPDATE: &str = "PerformLiveUpdate";
pub const OP_IDENTIFY_CAPABILITY: &str = "IdentifyCapability";
pub const OP_HEALTH_REQUEST: &str = "HealthRequest";
pub const OP_INITIALIZE: &str = "Initialize";
pub const OP_BIND_ACTOR: &str = "BindActor";
pub const OP_REMOVE_ACTOR: &str = "RemoveActor";

impl Supervised for MessageBus {}

impl SystemService for MessageBus {
    fn service_started(&mut self, ctx: &mut Context<Self>) {
        info!("Message Bus started");

        // TODO: make this value configurable
        ctx.set_mailbox_capacity(1000);
        self.hb(ctx);
    }
}

impl HostLocalSystemService for MessageBus {}

impl Actor for MessageBus {
    type Context = Context<Self>;
}

impl Handler<FindBindings> for MessageBus {
    type Result = FindBindingsResponse;

    fn handle(&mut self, msg: FindBindings, _ctx: &mut Context<Self>) -> Self::Result {
        let res = self
            .binding_cache
            .find_bindings(&msg.binding_name, &msg.provider_id);
        FindBindingsResponse { bindings: res }
    }
}

impl Handler<QueryActors> for MessageBus {
    type Result = QueryResponse;

    fn handle(&mut self, _msg: QueryActors, _ctx: &mut Context<Self>) -> QueryResponse {
        QueryResponse {
            results: self
                .subscribers
                .keys()
                .filter_map(|k| match k {
                    WasccEntity::Actor(s) => Some(s.to_string()),
                    WasccEntity::Capability { .. } => None,
                })
                .collect(),
        }
    }
}

// Receive a notification of claims
impl Handler<PutClaims> for MessageBus {
    type Result = ();

    fn handle(&mut self, msg: PutClaims, _ctx: &mut Context<Self>) {
        self.claims_cache
            .insert(msg.claims.subject.to_string(), msg.claims);
    }
}

// Receive a link definition through an advertisement
impl Handler<PutLink> for MessageBus {
    type Result = ();

    fn handle(&mut self, msg: PutLink, _ctx: &mut Context<Self>) {
        trace!("Messagebus received link definition notification");
        self.binding_cache.add_binding(
            &msg.actor,
            &msg.contract_id,
            &msg.binding_name,
            &msg.provider_id,
            msg.values.clone(),
        );
    }
}

impl Handler<CanInvoke> for MessageBus {
    type Result = bool;

    fn handle(&mut self, msg: CanInvoke, _ctx: &mut Context<Self>) -> Self::Result {
        let c = self.claims_cache.get(&msg.actor);
        if c.is_none() {
            return false;
        }
        let c = c.unwrap();
        let target = WasccEntity::Capability {
            id: msg.provider_id,
            contract_id: msg.contract_id.to_string(),
            binding: msg.link_name,
        };
        let pre_auth = if let Some(ref a) = c.metadata {
            if let Some(ref c) = a.caps {
                c.contains(&msg.contract_id)
            } else {
                false
            }
        } else {
            false
        };
        if !pre_auth {
            return false;
        }
        self.authorizer
            .as_ref()
            .unwrap()
            .can_invoke(c, &target, OP_BIND_ACTOR)
    }
}

impl Handler<QueryAllLinks> for MessageBus {
    type Result = LinksResponse;

    fn handle(&mut self, _msg: QueryAllLinks, _ctx: &mut Context<Self>) -> Self::Result {
        let lds = self
            .binding_cache
            .all()
            .iter()
            .map(|(k, v)| LinkDefinition {
                actor_id: k.actor.to_string(),
                provider_id: v.provider_id.to_string(),
                contract_id: k.contract_id.to_string(),
                link_name: k.binding_name.to_string(),
                values: v.values.clone(),
            })
            .collect();

        LinksResponse { links: lds }
    }
}

impl Handler<QueryProviders> for MessageBus {
    type Result = QueryResponse;

    fn handle(&mut self, _msg: QueryProviders, _ctx: &mut Context<Self>) -> QueryResponse {
        QueryResponse {
            results: self
                .subscribers
                .keys()
                .filter_map(|k| match k {
                    WasccEntity::Capability { id, .. } => Some(id.to_string()),
                    _ => None,
                })
                .collect(),
        }
    }
}

impl Handler<Initialize> for MessageBus {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: Initialize, ctx: &mut Context<Self>) -> Self::Result {
        self.key = Some(msg.key);
        self.authorizer = Some(msg.auth);
        self.nc = msg.nc;
        self.namespace = msg.namespace;
        let ns = self.namespace.clone();
        let timeout = msg.rpc_timeout.clone();
        info!("Messagebus initialized");
        if let Some(nc) = self.nc.clone() {
            let rpc_outbound = RpcClient::default().start();
            self.rpc_outbound = Some(rpc_outbound);
            let target = self.rpc_outbound.clone().unwrap();
            let bus = ctx.address().clone();
            let host_id = self.key.as_ref().unwrap().public_key();
            info!("Messagebus initializing with lattice RPC support");
            Box::pin(
                async move {
                    let _ = target
                        .send(super::rpc_client::Initialize {
                            host_id,
                            nc: Arc::new(nc),
                            ns_prefix: ns,
                            bus,
                            rpc_timeout: timeout,
                        })
                        .await;
                }
                .into_actor(self),
            )
        } else {
            Box::pin(async move {}.into_actor(self))
        }
    }
}

impl Handler<AdvertiseBinding> for MessageBus {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, msg: AdvertiseBinding, _ctx: &mut Context<Self>) -> Self::Result {
        if !self.claims_cache.contains_key(&msg.actor.to_string()) {}
        trace!("Advertisting link definition");
        let target = WasccEntity::Capability {
            id: msg.provider_id.to_string(),
            contract_id: msg.contract_id.to_string(),
            binding: msg.binding_name.to_string(),
        };

        self.binding_cache.add_binding(
            &msg.actor,
            &msg.contract_id,
            &msg.binding_name,
            &msg.provider_id,
            msg.values.clone(),
        );

        let advbinding = msg.clone();

        if let Some(t) = self.subscribers.get(&target) {
            let claims = self.claims_cache.get(&msg.actor.to_string()).unwrap();
            let req = super::utils::generate_binding_invocation(
                t,
                &msg.actor,
                msg.values.clone(),
                self.key.as_ref().unwrap(),
                target,
                claims.clone(),
            );
            Box::pin(req.into_actor(self).map(move |res, _act, _ctx| match res {
                Ok(ir) => {
                    if let Some(er) = ir.error {
                        Err(format!("Failed to set binding: {}", er).into())
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err("Mailbox error setting binding".into()),
            }))
        } else {
            // No _local_ subscriber found for this target.
            let rpc = self.rpc_outbound.clone();
            Box::pin( async move {
                if let Some(ref rpc) = rpc {
                    let _ = rpc.send(advbinding).await;
                } else {
                    info!("No potential targets for advertised link definition, no lattice RPC enabled. Assuming this provider will be added later.");
                }
                Ok(())
            }.into_actor(self))
        }
    }
}

impl Handler<AdvertiseClaims> for MessageBus {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, msg: AdvertiseClaims, _ctx: &mut Context<Self>) -> Self::Result {
        trace!("Advertising claims");
        self.claims_cache
            .insert(msg.claims.subject.to_string(), msg.claims.clone());

        let rpc = self.rpc_outbound.clone();
        if let Some(rpc) = rpc {
            Box::pin(
                async move {
                    let _ = rpc.send(msg).await;
                    Ok(())
                }
                .into_actor(self),
            )
        } else {
            Box::pin(async move { Ok(()) }.into_actor(self))
        }
    }
}

impl Handler<Invocation> for MessageBus {
    type Result = ResponseActFuture<Self, InvocationResponse>;

    /// Handle an invocation from any source to any target. If there is a local subscriber
    /// then the invocation will be delivered directly to that subscriber. If the subscriber
    /// is not local, _and_ there is a lattice provider configured, then the bus will attempt
    /// to satisfy that call via RPC over lattice.
    fn handle(&mut self, msg: Invocation, _ctx: &mut Context<Self>) -> Self::Result {
        trace!(
            "{}: Handling invocation from {} to {}",
            self.key.as_ref().unwrap().public_key(),
            msg.origin_url(),
            msg.target_url()
        );
        if let Err(e) = auth::authorize_invocation(
            &msg,
            self.authorizer.as_ref().unwrap().clone(),
            &self.claims_cache,
        ) {
            error!("Authorization failure: {}", e);
            return Box::pin(
                async move {
                    InvocationResponse::error(&msg, &format!("Authorization denied: {}", e))
                }.into_actor(self)
            );
        }
        let subscribers = self.subscribers.clone();
        match subscribers.get(&msg.target) {
            Some(target) => {
                trace!("Invocation taking place within bus");
                Box::pin(
                    target
                        .send(msg.clone())
                        .into_actor(self)
                        .map(move |res, _act, _ctx| {
                            if let Ok(r) = res {
                                r
                            } else {
                                InvocationResponse::error(
                                    &msg,
                                    "Mailbox error attempting to perform invocation",
                                )
                            }
                        }),
                )
            }
            None => {
                if self.rpc_outbound.is_none() {
                    warn!("No local subscribers and no RPC client enabled - invocation lost");
                    Box::pin(
                        async move {
                            InvocationResponse::error(
                            &msg,
                            &"No local bus subscribers found, and no lattice RPC client enabled",
                        )
                        }
                        .into_actor(self),
                    )
                } else {
                    trace!("Deferring invocation to lattice (no local subscribers)");
                    let rpc = self.rpc_outbound.clone().unwrap();
                    Box::pin(
                        async move {
                            let ir = rpc.send(msg.clone()).await;
                            match ir {
                                Ok(ir) => ir,
                                Err(e) => InvocationResponse::error(
                                    &msg,
                                    &format!("Error performing lattice RPC {:?}", e),
                                ),
                            }
                        }
                        .into_actor(self),
                    )
                }
            }
        }
    }
}

impl Handler<LookupBinding> for MessageBus {
    type Result = Option<String>;

    fn handle(&mut self, msg: LookupBinding, _ctx: &mut Self::Context) -> Self::Result {
        self.binding_cache
            .find_provider_id(&msg.actor, &msg.contract_id, &msg.binding_name)
    }
}

// register interest for an entity that's "on" the bus. if the bus has a
// nats connection, it will register the interest of an RPC subscription proxy. If there is no
// nats connection, it will register the interest of the actual subscriber.
impl Handler<Subscribe> for MessageBus {
    type Result = ResponseActFuture<Self, ()>;

    fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) -> Self::Result {
        trace!("Bus registered interest for {}", &msg.interest.url());

        let nc = self.nc.clone();
        let ns = self.namespace.clone();
        Box::pin(
            async move {
                let interest = msg.interest.clone();
                let address = if let Some(ref nc) = nc {
                    let addr = RpcSubscription::default().start();
                    let _ = addr
                        .send(CreateSubscription {
                            entity: msg.interest.clone(),
                            target: msg.subscriber,
                            nc: Arc::new(nc.clone()),
                            namespace: ns,
                        })
                        .await;
                    addr.recipient() // RPC subscriber proxy
                } else {
                    msg.subscriber // Actual subscriber
                };
                (interest, address)
            }
            .into_actor(self)
            .map(|(entity, res), act, _ctx| {
                act.subscribers.insert(entity, res);
            }),
        )
    }
}

impl Handler<Unsubscribe> for MessageBus {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe, _ctx: &mut Context<Self>) {
        trace!("Bus removing interest for {}", msg.interest.url());
        if let None = self.subscribers.remove(&msg.interest) {
            warn!("Attempted to remove a non-existent subscriber");
        }
    }
}

impl Handler<GetClaims> for MessageBus {
    type Result = ClaimsResponse;

    fn handle(&mut self, _msg: GetClaims, _ctx: &mut Context<Self>) -> Self::Result {
        ClaimsResponse {
            claims: self.claims_cache.clone(),
        }
    }
}
