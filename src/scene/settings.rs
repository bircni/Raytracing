use serde::{Deserialize, Serialize};

// This struct represents settings for a program and is both serializable and deserializable
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    
    // This field specifies the maximum number of bounces for ray tracing or similar operations
    max_bounces: usize,

    // This field represents the number of samples used in rendering or some other operation
    samples: usize,
}

