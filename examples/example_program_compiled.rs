#[unsafe(no_mangle)]
pub fn get_result(input: &str) -> Result<String, String> {
	Ok(input.to_uppercase())
}