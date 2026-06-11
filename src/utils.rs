/// Prints and returns the value of a given expression for quick and dirty
/// logging-based debugging.
///
/// This macro is a drop-in replacement for the standard library's `dbg!` macro,
/// but instead of printing to `stderr`, it routes output through `tracing`.
///
/// # Shorthand Severity Levels
///
/// You can prefix the macro with a single letter followed by a semicolon
/// `dbg_log!(I; foo)` to change the log severity:
/// * `E;` -> Error
/// * `W;` -> Warn
/// * `I;` -> Info (default)
/// * `D;` -> Debug
/// * `T;` -> Trace
///
/// # Examples
///
/// ```rust
/// // Logs "[src/main.rs:10:5] a * 2 = 4" at Info level
/// let a = 2;
/// let b = dbg_log!(a * 2);
///
/// // Log at a specific level using shorthand
/// dbg_log!(W; b); // Logs at Warning level
///
/// // Wrap an expression inline
/// let c = dbg_log!(D; b + 1) + 2;
///
/// // Multiple expressions return a tuple
/// let (x, y) = dbg_log!(E; a, c);
/// ```
#[macro_export]
macro_rules! dbg_log {
	// 1. Single-letter shorthand branch
	($level:ident; $($val:expr),+ $(,)?) => {
		$crate::dbg_log!(
			match stringify!($level) {
				"W" => ::tracing::Level::WARN,
				"E" => ::tracing::Level::ERROR,
				"T" => ::tracing::Level::TRACE,
				"D" => ::tracing::Level::DEBUG,
				_ => ::tracing::Level::INFO, // Default for "I" or anything else
			};
			$($val),+
		)
	};

	// 2. Full Level path branch (allows Level::Warn; expr)
	($level:expr; $($val:expr),+ $(,)?) => {
		($({
			match $val {
				tmp => {
					::tracing::event!($level, "[{}:{}:{}] {} = {:#?}",
						file!(), line!(), column!(), stringify!($val), &tmp);
					tmp
				}
			}
		}),+)
	};

	// 3. Default branch (no level specified)
	($($val:expr),+ $(,)?) => {
		$crate::dbg_log!(::tracing::Level::INFO; $($val),+)
	};

	// 4. Empty call
	() => { ::tracing::info!("[{}:{}:{}]", file!(), line!(), column!()) };
}
