/// this generates a Rpc contract with the shape
/// ```json
/// {
///		"method": "String"
///     "data": { ... }
/// }
/// ```
#[rpissc::rpc]
pub struct Rpc {
    #[rpiss::method]
    pub method: String,
}

#[rpissc::rpc]
pub struct CoffeeServer {
	pub db: Pool<Sqlite>,
}

#[rpissc::service]
trait CoffeeRpc {
	type Shape = Rpc;
	async fn get_random_coffee(&self) -> anyhow::Result<Coffee>;
}

impl CoffeeRpc for CoffeeServer {
	/// ... 
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
	// ...
	let server = CoffeeServer { db: pool };
	server.serve().await?;
}