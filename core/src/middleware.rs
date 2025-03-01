// Copyright 2019-2021 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any
// person obtaining a copy of this software and associated
// documentation files (the "Software"), to deal in the
// Software without restriction, including without
// limitation the rights to use, copy, modify, merge,
// publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software
// is furnished to do so, subject to the following
// conditions:
//
// The above copyright notice and this permission notice
// shall be included in all copies or substantial portions
// of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
// ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
// TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
// PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
// SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
// CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
// OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
// IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Middleware for `jsonrpsee` servers.

use std::net::SocketAddr;

pub use http::HeaderMap as Headers;
pub use jsonrpsee_types::Params;

/// Defines a middleware specifically for HTTP requests with callbacks during the RPC request life-cycle.
/// The primary use case for this is to collect timings for a larger metrics collection solution.
///
/// See [`HttpServerBuilder::set_middleware`](../../jsonrpsee_http_server/struct.HttpServerBuilder.html#method.set_middleware) method
/// for examples.
pub trait HttpMiddleware: Send + Sync + Clone + 'static {
	/// Intended to carry timestamp of a request, for example `std::time::Instant`. How the middleware
	/// measures time, if at all, is entirely up to the implementation.
	type Instant: std::fmt::Debug + Send + Sync + Copy;

	/// Called when a new JSON-RPC request comes to the server.
	fn on_request(&self, remote_addr: SocketAddr, headers: &Headers) -> Self::Instant;

	/// Called on each JSON-RPC method call, batch requests will trigger `on_call` multiple times.
	fn on_call(&self, method_name: &str, params: Params);

	/// Called on each JSON-RPC method completion, batch requests will trigger `on_result` multiple times.
	fn on_result(&self, method_name: &str, success: bool, started_at: Self::Instant);

	/// Called once the JSON-RPC request is finished and response is sent to the output buffer.
	fn on_response(&self, result: &str, _started_at: Self::Instant);
}

/// Defines a middleware specifically for WebSocket connections with callbacks during the RPC request life-cycle.
/// The primary use case for this is to collect timings for a larger metrics collection solution.
///
/// See the [`WsServerBuilder::set_middleware`](../../jsonrpsee_ws_server/struct.WsServerBuilder.html#method.set_middleware)
/// for examples.
pub trait WsMiddleware: Send + Sync + Clone + 'static {
	/// Intended to carry timestamp of a request, for example `std::time::Instant`. How the middleware
	/// measures time, if at all, is entirely up to the implementation.
	type Instant: std::fmt::Debug + Send + Sync + Copy;

	/// Called when a new client connects
	fn on_connect(&self, remote_addr: SocketAddr, headers: &Headers);

	/// Called when a new JSON-RPC request comes to the server.
	fn on_request(&self) -> Self::Instant;

	/// Called on each JSON-RPC method call, batch requests will trigger `on_call` multiple times.
	fn on_call(&self, method_name: &str, params: Params);

	/// Called on each JSON-RPC method completion, batch requests will trigger `on_result` multiple times.
	fn on_result(&self, method_name: &str, success: bool, started_at: Self::Instant);

	/// Called once the JSON-RPC request is finished and response is sent to the output buffer.
	fn on_response(&self, result: &str, started_at: Self::Instant);

	/// Called when a client disconnects
	fn on_disconnect(&self, remote_addr: std::net::SocketAddr);
}

impl HttpMiddleware for () {
	type Instant = ();

	fn on_request(&self, _: std::net::SocketAddr, _: &Headers) -> Self::Instant {}

	fn on_call(&self, _: &str, _: Params) {}

	fn on_result(&self, _: &str, _: bool, _: Self::Instant) {}

	fn on_response(&self, _: &str, _: Self::Instant) {}
}

impl WsMiddleware for () {
	type Instant = ();

	fn on_connect(&self, _: std::net::SocketAddr, _: &Headers) {}

	fn on_request(&self) -> Self::Instant {}

	fn on_call(&self, _: &str, _: Params) {}

	fn on_result(&self, _: &str, _: bool, _: Self::Instant) {}

	fn on_response(&self, _: &str, _: Self::Instant) {}

	fn on_disconnect(&self, _: std::net::SocketAddr) {}
}

impl<A, B> WsMiddleware for (A, B)
where
	A: WsMiddleware,
	B: WsMiddleware,
{
	type Instant = (A::Instant, B::Instant);

	fn on_connect(&self, remote_addr: std::net::SocketAddr, headers: &Headers) {
		(self.0.on_connect(remote_addr, headers), self.1.on_connect(remote_addr, headers));
	}

	fn on_request(&self) -> Self::Instant {
		(self.0.on_request(), self.1.on_request())
	}

	fn on_call(&self, method_name: &str, params: Params) {
		self.0.on_call(method_name, params.clone());
		self.1.on_call(method_name, params);
	}

	fn on_result(&self, method_name: &str, success: bool, started_at: Self::Instant) {
		self.0.on_result(method_name, success, started_at.0);
		self.1.on_result(method_name, success, started_at.1);
	}

	fn on_response(&self, result: &str, started_at: Self::Instant) {
		self.0.on_response(result, started_at.0);
		self.1.on_response(result, started_at.1);
	}

	fn on_disconnect(&self, remote_addr: std::net::SocketAddr) {
		(self.0.on_disconnect(remote_addr), self.1.on_disconnect(remote_addr));
	}
}

impl<A, B> HttpMiddleware for (A, B)
where
	A: HttpMiddleware,
	B: HttpMiddleware,
{
	type Instant = (A::Instant, B::Instant);

	fn on_request(&self, remote_addr: std::net::SocketAddr, headers: &Headers) -> Self::Instant {
		(self.0.on_request(remote_addr, headers), self.1.on_request(remote_addr, headers))
	}

	fn on_call(&self, method_name: &str, params: Params) {
		self.0.on_call(method_name, params.clone());
		self.1.on_call(method_name, params);
	}

	fn on_result(&self, method_name: &str, success: bool, started_at: Self::Instant) {
		self.0.on_result(method_name, success, started_at.0);
		self.1.on_result(method_name, success, started_at.1);
	}

	fn on_response(&self, result: &str, started_at: Self::Instant) {
		self.0.on_response(result, started_at.0);
		self.1.on_response(result, started_at.1);
	}
}
