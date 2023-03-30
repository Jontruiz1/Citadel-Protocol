use citadel_sdk::prelude::*;

fn main(){
    
}

/*
use citadel_sdk::prelude::*;
use citadel_sdk::prefabs::server::empty::EmptyKernel;

// this server will listen on 127.0.0.1:25021, and will use the built-in defaults. When calling 'build', a NetKernel is specified
let server = NodeBuilder::default()
.with_node_type(NodeType::server("127.0.0.1:25021")?)
.build(EmptyKernel::default())?;

// await the server to execute
let result = server.await;
*/