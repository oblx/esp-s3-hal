use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::hal::led::LedState;

pub type LedMutex = Mutex<CriticalSectionRawMutex, LedState>;

/// CORS headers for cross-origin browser access (dashboard → device direct).
#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
const CORS_HEADERS: [(&str, &str); 3] = [
	("Access-Control-Allow-Origin", "*"),
	("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
	("Access-Control-Allow-Headers", "Content-Type"),
];

/// Wrapper that adds CORS headers to any response content.
/// Usage: `async fn handler() -> CorsResponse<String> { CorsResponse(json_string) }`
#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
pub struct CorsResponse<T: picoserve::response::Content>(pub T);

#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
impl<T: picoserve::response::Content> picoserve::response::IntoResponse for CorsResponse<T> {
	async fn write_to<R: picoserve::io::Read, W: picoserve::response::ResponseWriter<Error = R::Error>>(
		self,
		connection: picoserve::response::Connection<'_, R>,
		response_writer: W,
	) -> Result<picoserve::ResponseSent, W::Error> {
		response_writer
			.write_response(
				connection,
				picoserve::response::Response::new(picoserve::response::StatusCode::OK, self.0)
					.with_headers(CORS_HEADERS),
			)
			.await
	}
}

/// Response for OPTIONS preflight requests — CORS headers, no body.
#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
pub struct CorsPreflight;

#[cfg(any(feature = "wifi-ap", feature = "wifi-sta"))]
impl picoserve::response::IntoResponse for CorsPreflight {
	async fn write_to<R: picoserve::io::Read, W: picoserve::response::ResponseWriter<Error = R::Error>>(
		self,
		connection: picoserve::response::Connection<'_, R>,
		response_writer: W,
	) -> Result<picoserve::ResponseSent, W::Error> {
		response_writer
			.write_response(
				connection,
				picoserve::response::Response::empty(picoserve::response::StatusCode::NO_CONTENT)
					.with_headers(CORS_HEADERS),
			)
			.await
	}
}
