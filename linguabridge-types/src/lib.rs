//! Protobuf definitions for Akash Network and Cosmos SDK.
//!
//! This crate contains the `.proto`-generated Rust types for interacting with
//! Akash Network deployments, the Cosmos SDK, and Tendermint.

#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::unwrap_used)]
#![allow(non_snake_case)]

pub use prost::{Message, Name};

pub mod akash {
    pub mod base {
        pub mod v1beta3 {
            include!("gen/akash.base.v1beta3.rs");
        }
    }
    pub mod deployment {
        pub mod v1beta3 {
            include!("gen/akash.deployment.v1beta3.rs");
        }
    }
    pub mod escrow {
        pub mod v1beta3 {
            include!("gen/akash.escrow.v1beta3.rs");
        }
    }
    pub mod manifest {
        pub mod v2beta2 {
            include!("gen/akash.manifest.v2beta2.rs");
        }
    }
    pub mod market {
        pub mod v1beta4 {
            include!("gen/akash.market.v1beta4.rs");
        }
    }
    pub mod provider {
        pub mod lease {
            pub mod v1 {
                include!("gen/akash.provider.lease.v1.rs");
            }
        }
    }
}

pub mod cosmos {
    pub mod base {
        pub mod v1beta1 {
            include!("gen/cosmos.base.v1beta1.rs");
        }
        pub mod query {
            pub mod v1beta1 {
                include!("gen/cosmos.base.query.v1beta1.rs");
            }
        }
    }
    pub mod v1 {
        include!("gen/cosmos_proto.rs");
    }
}

pub mod tendermint {
    pub mod crypto {
        include!("gen/tendermint.crypto.rs");
    }

    #[allow(clippy::large_enum_variant)]
    pub mod types {
        include!("gen/tendermint.types.rs");
    }

    pub mod version {
        include!("gen/tendermint.version.rs");
    }

    pub mod p2p {
        include!("gen/tendermint.p2p.rs");
    }

    pub mod abci {
        include!("gen/tendermint.abci.rs");
    }
}
