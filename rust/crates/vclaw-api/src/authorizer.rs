pub trait ToolAuthorizer {
    fn authorize(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        side_effect_level: &str,
        call_count: usize,
    ) -> Result<(), String>;
}
